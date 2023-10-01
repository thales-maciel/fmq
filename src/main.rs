extern crate serde;
extern crate serde_yaml;
use clap::Parser;
use serde_yaml::{Mapping, Value};
use std::{fs::read_to_string, path::PathBuf};

fn extract_front_matter(input: &str) -> Option<String> {
    if !input.starts_with("---") {
        return None;
    }

    let mut lines = input.lines();
    lines.next();

    let mut front_matter = String::new();
    for line in lines {
        if line == "---" {
            return Some(front_matter);
        }
        front_matter.push_str(line);
        front_matter.push('\n');
    }
    None
}

type Metadata = Mapping;

fn parse_front_matter(fm: &str) -> Result<Metadata, serde_yaml::Error> {
    serde_yaml::from_str(fm)
}

struct Args {
    pub select: Option<Vec<Query>>,
    pub condition: Option<Condition>,
    pub sort_by: Option<String>,
    pub paths: Vec<PathBuf>,
}

#[derive(Debug)]
enum Ops {
    Eq,
    Neq,
    Gt,
    Gte,
    Lt,
    Lte,
}

impl TryFrom<&str> for Ops {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "==" => Ok(Self::Eq),
            "!=" => Ok(Self::Neq),
            ">" => Ok(Self::Gt),
            ">=" => Ok(Self::Gte),
            "<" => Ok(Self::Lt),
            "<=" => Ok(Self::Lte),
            v => Err(format!("No matching operator for {}", v)),
        }
    }
}

struct Condition {
    query: Query,
    op: Ops,
    value: String,
}

impl TryFrom<&str> for Condition {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let mut parts = value.splitn(3, ' ');
        let query_str = parts.next().ok_or("Missing Query")?;
        let op_str = parts.next().ok_or("Missing operator part")?;
        let value_str = parts.next().ok_or("Missing value part")?;

        let query = Query::try_from(query_str.to_string())?;
        let op = Ops::try_from(op_str)?;
        let value = value_str.to_string();

        Ok(Self { query, op, value })
    }
}

struct SourceFile {
    path: PathBuf,
    metadata: Metadata,
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    pub files: Vec<PathBuf>,

    #[arg(short, long)]
    pub select: Option<String>,

    #[arg(short, long)]
    pub condition: Option<String>,

    #[arg(short, long)]
    pub order_by: Option<String>,
}

fn main() {
    let cli = Cli::parse();

    // validate paths are not empty
    if cli.files.is_empty() {
        panic!("at least one file required")
    }
    // validate every path is a file
    for p in &cli.files {
        if !p.is_file() {
            panic!("path is not a file")
        }
    }

    // Validate arguments:
    let args = Args {
        select: cli.select.map(|v| {
            v.split(' ')
                .map(|s| Query::try_from(s.to_string()).unwrap())
                .collect()
        }),
        condition: cli
            .condition
            .map(|cond| Condition::try_from(cond.as_str()).unwrap()),
        sort_by: cli.order_by,
        paths: cli.files,
    };

    // Create a place to put the results into
    let mut processed: Vec<SourceFile> = vec![];

    // > For each file:
    for path in args.paths {
        let file_contents = read_to_string(&path).unwrap();
        let Some(Ok(fm)) = extract_front_matter(&file_contents).map(|s| parse_front_matter(&s))
        else {
            continue;
        };

        if let Some(cond) = &args.condition {
            if let Some(field) = get_value(&cond.query, &fm) {
                if let Some(s) = field.as_str() {
                    match cond.op {
                        Ops::Eq => {
                            if s != cond.value {
                                continue;
                            }
                        }
                        Ops::Neq => {
                            if s == cond.value {
                                continue;
                            }
                        }
                        Ops::Gt => {
                            if s <= cond.value.as_str() {
                                continue;
                            }
                        }
                        Ops::Gte => {
                            if s < cond.value.as_str() {
                                continue;
                            }
                        }
                        Ops::Lt => {
                            if s >= cond.value.as_str() {
                                continue;
                            }
                        }
                        Ops::Lte => {
                            if s > cond.value.as_str() {
                                continue;
                            }
                        }
                    }
                }
            } else {
                continue;
            }
        }
        processed.push(SourceFile { path, metadata: fm });
    }

    // > TODO: join back to the main thread

    // > do we have to sort?
    if let Some(sort_by) = args.sort_by {
        processed.sort_by(|a, b| {
            let a = a.metadata.get(&sort_by).unwrap_or(&serde_yaml::Value::Null);
            let b = b.metadata.get(&sort_by).unwrap_or(&serde_yaml::Value::Null);
            a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)
        })
    };

    if let Some(queries) = &args.select {
        for res in processed {
            let mut values: Vec<String> = vec![];
            for query in queries {
                let raw_value = get_value(query, &res.metadata);
                match raw_value {
                    Some(raw) => {
                        let mut val = serde_yaml::to_string(&raw).unwrap_or("".into());
                        val.pop();
                        values.push(val);
                    }
                    None => values.push("null".into()),
                }
            }
            let escaped_values: String = values.join(", ").escape_default().collect();
            println!("{}, {}", res.path.display(), escaped_values);
        }
    } else {
        for res in processed {
            let mut val = serde_yaml::to_string(&res.metadata).unwrap();
            val.pop();
            println!("file: {}\n{}", res.path.display(), val);
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum QueryAccessor {
    Key(String),
    Index(usize),
}

#[derive(Debug, PartialEq, Eq)]
pub struct Query(Vec<QueryAccessor>);

impl TryFrom<String> for Query {
    type Error = String;

    fn try_from(input: String) -> Result<Self, Self::Error> {
        let mut res = Vec::new();
        let mut chars = input.chars().peekable();
        let mut current_key = String::new();

        while let Some(c) = chars.next() {
            match c {
                '.' => {
                    if !current_key.is_empty() {
                        res.push(QueryAccessor::Key(current_key.clone()));
                        current_key.clear();
                    }
                }
                '[' => {
                    if !current_key.is_empty() {
                        res.push(QueryAccessor::Key(current_key.clone()));
                        current_key.clear();
                    }
                    let mut index_str = String::new();
                    while let Some(d) = chars.peek() {
                        if *d == ']' {
                            chars.next();
                            break;
                        }
                        index_str.push(chars.next().unwrap());
                    }
                    if index_str.is_empty() {
                        return Err("Unclosed bracket or empty index".to_string());
                    }
                    if let Ok(index) = index_str.parse::<usize>() {
                        res.push(QueryAccessor::Index(index));
                    } else {
                        return Err(format!("Invalid index: {}", index_str));
                    }
                }
                _ => current_key.push(c),
            }
        }

        if !current_key.is_empty() {
            res.push(QueryAccessor::Key(current_key));
        }

        Ok(Self(res))
    }
}

pub fn get_value(query: &Query, metadata: &Metadata) -> Option<Value> {
    let mut current_value = Value::from(metadata.to_owned());

    for accessor in &query.0 {
        current_value = match accessor {
            QueryAccessor::Key(key) => current_value[key].clone(),
            QueryAccessor::Index(idx) => {
                if let Some(arr) = current_value.as_sequence() {
                    arr.get(idx.to_owned()).unwrap_or(&Value::Null).clone()
                } else {
                    return None;
                }
            }
        }
    }
    Some(current_value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_value() {
        let query = Query::try_from("key".to_string()).unwrap();
        let mut meta = Mapping::new();
        meta.insert(
            Value::String("key".to_string()),
            Value::String("value".to_string()),
        );
        let result = get_value(&query, &meta);
        assert_eq!(result, Some(Value::String("value".to_string())))
    }

    #[test]
    fn test_get_value_nested() {
        let query = Query::try_from("key.nested".to_string()).unwrap();
        let mut meta = Mapping::new();
        let mut nested = Mapping::new();
        nested.insert(
            Value::String("nested".to_string()),
            Value::String("value".to_string()),
        );
        meta.insert(Value::String("key".to_string()), Value::Mapping(nested));
        let result = get_value(&query, &meta);
        assert_eq!(result, Some(Value::String("value".to_string())))
    }

    #[test]
    fn test_single_key() {
        let query = Query::try_from("key".to_string()).unwrap();
        assert_eq!(query.0, vec![QueryAccessor::Key("key".to_string())]);
    }

    #[test]
    fn test_nested_key() {
        let query = Query::try_from("key.nested".to_string()).unwrap();
        assert_eq!(
            query.0,
            vec![
                QueryAccessor::Key("key".to_string()),
                QueryAccessor::Key("nested".to_string())
            ]
        );
    }

    #[test]
    fn test_key_with_index() {
        let query = Query::try_from("key[0]".to_string()).unwrap();
        assert_eq!(
            query.0,
            vec![
                QueryAccessor::Key("key".to_string()),
                QueryAccessor::Index(0)
            ]
        );
    }

    #[test]
    fn test_nested_key_with_index() {
        let query = Query::try_from("key.nested[0]".to_string());
        match query {
            Ok(q) => assert_eq!(
                q.0,
                vec![
                    QueryAccessor::Key("key".to_string()),
                    QueryAccessor::Key("nested".to_string()),
                    QueryAccessor::Index(0)
                ]
            ),
            Err(s) => panic!("{}", s),
        }
    }

    #[test]
    fn test_invalid_index() {
        let query = Query::try_from("key.nested[invalid_index]".to_string());
        assert!(query.is_err());
    }

    #[test]
    fn test_unclosed_bracket() {
        let query = Query::try_from("key.nested[".to_string());
        assert!(query.is_err());
    }
}
