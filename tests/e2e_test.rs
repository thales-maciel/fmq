use std::path::PathBuf;

use assert_cmd::Command;

pub enum File {
    Full,
    Partial,
    None,
    Missing,
    Invalid,
}

fn path(file: File) -> PathBuf {
    match file {
        File::Full => "./tests/fixtures/full.md".try_into().unwrap(),
        File::Partial => "./tests/fixtures/partial.md".try_into().unwrap(),
        File::None => "./tests/fixtures/no-fm.md".try_into().unwrap(),
        File::Missing => "./tests/fixtures/missing.md".try_into().unwrap(),
        File::Invalid => "./tests/fixtures/invalid.md".try_into().unwrap(),
    }
}

#[test]
fn dry_run() {
    let mut cmd = Command::cargo_bin("fmq").unwrap();
    let assert = cmd.assert();
    assert.failure();
}

#[test]
fn happy() {
    let mut cmd = Command::cargo_bin("fmq").unwrap();
    let assert = cmd.arg(path(File::Full)).assert();
    assert
        .success()
        .stdout("./tests/fixtures/full.md, 2023-09-26, asdf jkl, 0, Lorem Ipsum\n");
}

#[test]
fn select() {
    let mut cmd = Command::cargo_bin("fmq").unwrap();
    let assert = cmd
        .args(&["-s", "title date missing"])
        .arg(path(File::Full))
        .assert();
    assert
        .success()
        .stdout("./tests/fixtures/full.md, Lorem Ipsum, 2023-09-26, null\n");
}

#[test]
fn sort() {
    let mut cmd = Command::cargo_bin("fmq").unwrap();
    let assert = cmd
        .args(&["-s", "title date", "-o", "title"])
        .arg(path(File::Full))
        .arg(path(File::Partial))
        .assert();
    assert
        .success()
        .stdout("./tests/fixtures/partial.md, Another Title, 2023-09-27\n./tests/fixtures/full.md, Lorem Ipsum, 2023-09-26\n");
}

#[test]
fn other_sort() {
    let mut cmd = Command::cargo_bin("fmq").unwrap();
    let assert = cmd
        .args(&["-s", "title date", "-o", "date"])
        .arg(path(File::Full))
        .arg(path(File::Partial))
        .assert();
    assert
        .success()
        .stdout("./tests/fixtures/full.md, Lorem Ipsum, 2023-09-26\n./tests/fixtures/partial.md, Another Title, 2023-09-27\n");
}

#[test]
fn condition() {
    let mut cmd = Command::cargo_bin("fmq").unwrap();
    let assert = cmd
        .args(&["-s", "title date", "-c", "title"])
        .arg(path(File::Full))
        .arg(path(File::Partial))
        .arg(path(File::Invalid))
        .arg(path(File::None))
        .assert();
    assert
        .success()
        .stdout("./tests/fixtures/full.md, Lorem Ipsum, 2023-09-26\n./tests/fixtures/partial.md, Another Title, 2023-09-27\n");
}

#[test]
fn no_args() {
    let mut cmd = Command::cargo_bin("fmq").unwrap();
    let assert = cmd.arg(path(File::Full)).assert();
    assert.success();

    let mut cmd = Command::cargo_bin("fmq").unwrap();
    let assert = cmd.arg(path(File::Partial)).assert();
    assert.success();

    let mut cmd = Command::cargo_bin("fmq").unwrap();
    let assert = cmd.arg(path(File::Missing)).assert();
    assert.failure();

    let mut cmd = Command::cargo_bin("fmq").unwrap();
    let assert = cmd.arg(path(File::None)).assert();
    assert.success();

    let mut cmd = Command::cargo_bin("fmq").unwrap();
    let assert = cmd.arg(path(File::Invalid)).assert();
    assert.success();
}
