extern crate serde;
extern crate serde_yaml;
use std::{collections::HashMap, path::PathBuf, fs::read_to_string};
use clap::Parser;
use itertools::Itertools;

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

type Metadata = HashMap<String, serde_yaml::Value>;

fn parse_front_matter(fm: &str) -> Result<Metadata, serde_yaml::Error> {
    serde_yaml::from_str(fm)
}

struct Args {
    pub select: Option<Vec<String>>,
    pub condition: Option<String>,
    pub sort_by: Option<String>,
    pub paths: Vec<PathBuf>,
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
    if cli.files.is_empty() { panic!("at least one file required") }
    // validate every path is a file
    for p in &cli.files { if !p.is_file() { panic!("path is not a file") } }

    // Validate arguments:
    let args = Args{
        select: cli.select.map(|v| v.split(" ").map(|s| s.to_string()).collect()),
        condition: cli.condition,
        sort_by: cli.order_by,
        paths: cli.files,
    };

    // Create a place to put the results into
    let mut processed: Vec<SourceFile> = vec!();

    // > Parallelize it. For each file:
    for path in args.paths {
        let file_contents = read_to_string(&path).unwrap();
        let Some(Ok(fm)) = extract_front_matter(&file_contents)
            .map(|s| parse_front_matter(&s)) else { continue; };

        if let Some(cond) = &args.condition {
            // TODO: parse the condition query
            if fm.get(cond).is_none() { continue; }
        }
        processed.push(SourceFile { path, metadata: fm });
    }

    // > TODO: join back to the main thread

    // > do we have to sort?
    if let Some(sort_by) = args.sort_by {
        processed
            .sort_by(|a, b| {
                let ay = a.metadata.get(&sort_by).unwrap_or(&serde_yaml::Value::Null);
                let by = b.metadata.get(&sort_by).unwrap_or(&serde_yaml::Value::Null);
                let lel = ay.partial_cmp(&by).unwrap_or(std::cmp::Ordering::Equal);
                lel
            })
    };

    if let Some(fields) = &args.select {
        for res in processed {
            let mut values: Vec<String> = vec!();
            for field in fields {
                let raw_value = res.metadata.get(field);
                match raw_value {
                    Some(raw) => {
                        let mut val = serde_yaml::to_string(raw).unwrap_or("".into());
                        val.pop();
                        values.push(val);
                    },
                    None => values.push("null".into()),
                }
            }
            let escaped_values: String = values.join(", ").escape_default().collect();
            println!("{}, {}", res.path.display(), escaped_values);
        }
    } else {
        for res in processed {
            let mut values: Vec<String> = vec!();
            for k in res.metadata.keys().sorted() {
                let mut val = serde_yaml::to_string(res.metadata.get(k).unwrap()).unwrap_or("".into());
                val.pop();
                values.push(val);
            }
            let escaped_values: String = values.join(", ").escape_default().collect();
            println!("{}, {}", res.path.display(), escaped_values);
        }
    }
}

