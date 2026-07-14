//! End-to-end tests of the `boinc` binary.

#![allow(clippy::unwrap_used)]

use std::path::Path;

use assert_cmd::Command;
use predicates::prelude::*;

fn boinc() -> Command {
    Command::cargo_bin("boinc").unwrap()
}

fn write_test_png(path: &Path) {
    let mut img = image::RgbaImage::new(3, 2);
    for (i, pixel) in img.pixels_mut().enumerate() {
        *pixel = image::Rgba([(i * 40) as u8, 100, 200, 255]);
    }
    img.save(path).unwrap();
}

#[test]
fn convert_png_to_jpg() {
    let dir = tempfile::tempdir().unwrap();
    let png = dir.path().join("pic.png");
    write_test_png(&png);

    boinc()
        .args(["convert", png.to_str().unwrap(), "--to", "jpg"])
        .assert()
        .success()
        .stdout(predicate::str::contains("pic.jpg"));

    assert!(dir.path().join("pic.jpg").is_file());
}

#[test]
fn convert_batch_to_out_dir() {
    let dir = tempfile::tempdir().unwrap();
    let out = tempfile::tempdir().unwrap();
    let a = dir.path().join("a.png");
    let b = dir.path().join("b.png");
    write_test_png(&a);
    write_test_png(&b);

    boinc()
        .args([
            "convert",
            a.to_str().unwrap(),
            b.to_str().unwrap(),
            "--to",
            "jpg",
            "--out-dir",
            out.path().to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(out.path().join("a.jpg").is_file());
    assert!(out.path().join("b.jpg").is_file());
}

#[test]
fn convert_json_emits_events() {
    let dir = tempfile::tempdir().unwrap();
    let png = dir.path().join("pic.png");
    write_test_png(&png);

    let output = boinc()
        .args(["convert", png.to_str().unwrap(), "--to", "jpg", "--json"])
        .output()
        .unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).unwrap();
    let events: Vec<serde_json::Value> = stdout
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();

    assert!(
        events
            .iter()
            .any(|e| e["event"] == "progress" && e["percent"] == 100)
    );
    let converted = events
        .iter()
        .find(|e| e["event"] == "converted")
        .expect("a converted event");
    assert_eq!(converted["from"], "png");
    assert_eq!(converted["to"], "jpg");
    assert!(Path::new(converted["output"].as_str().unwrap()).is_file());
}

#[test]
fn batch_continues_after_failure_and_exits_nonzero() {
    let dir = tempfile::tempdir().unwrap();
    let good = dir.path().join("good.png");
    write_test_png(&good);
    let junk = dir.path().join("junk.png");
    std::fs::write(&junk, b"not an image").unwrap();

    boinc()
        .args([
            "convert",
            junk.to_str().unwrap(),
            good.to_str().unwrap(),
            "--to",
            "jpg",
        ])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("error"));

    // The good file was still converted despite the earlier failure.
    assert!(dir.path().join("good.jpg").is_file());
}

#[test]
fn missing_input_fails() {
    boinc()
        .args(["convert", "/nonexistent/file.png", "--to", "jpg"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("error"));
}

#[test]
fn out_with_multiple_inputs_is_usage_error() {
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("a.png");
    let b = dir.path().join("b.png");
    write_test_png(&a);
    write_test_png(&b);

    boinc()
        .args([
            "convert",
            a.to_str().unwrap(),
            b.to_str().unwrap(),
            "--to",
            "jpg",
            "--out",
            dir.path().join("out.jpg").to_str().unwrap(),
        ])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("--out"));
}

#[test]
fn explicit_out_refuses_to_overwrite() {
    let dir = tempfile::tempdir().unwrap();
    let png = dir.path().join("pic.png");
    write_test_png(&png);
    let taken = dir.path().join("taken.jpg");
    std::fs::write(&taken, b"precious").unwrap();

    boinc()
        .args([
            "convert",
            png.to_str().unwrap(),
            "--to",
            "jpg",
            "--out",
            taken.to_str().unwrap(),
        ])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("already exists"));

    assert_eq!(std::fs::read(&taken).unwrap(), b"precious");
}

#[test]
fn unknown_format_is_usage_error() {
    boinc()
        .args(["convert", "whatever.png", "--to", "gif"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("unknown format"));
}

#[test]
fn list_conversions_text() {
    boinc()
        .arg("list-conversions")
        .assert()
        .success()
        .stdout(predicate::str::contains("PNG -> JPG"))
        .stdout(predicate::str::contains("JPG -> PNG"));
}

#[test]
fn list_conversions_all_includes_document_pairs() {
    boinc()
        .args(["list-conversions", "--all"])
        .assert()
        .success()
        .stdout(predicate::str::contains("DOCX -> PDF"))
        .stdout(predicate::str::contains("PDF -> DOCX"));
}

#[test]
fn list_conversions_for_file() {
    let dir = tempfile::tempdir().unwrap();
    let png = dir.path().join("pic.png");
    write_test_png(&png);

    boinc()
        .args(["list-conversions", png.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("PNG -> JPG"));
}

#[test]
fn list_conversions_json() {
    let output = boinc()
        .args(["list-conversions", "--all", "--json"])
        .output()
        .unwrap();
    assert!(output.status.success());

    let list: Vec<serde_json::Value> =
        serde_json::from_slice(&output.stdout).expect("a JSON array");
    assert!(
        list.iter()
            .any(|row| row["from"] == "png" && row["to"] == "jpg" && row["available"] == true)
    );
    assert!(
        list.iter()
            .any(|row| row["from"] == "docx" && row["to"] == "pdf")
    );
}

#[test]
fn list_conversions_json_for_file() {
    let dir = tempfile::tempdir().unwrap();
    let png = dir.path().join("pic.png");
    write_test_png(&png);

    let output = boinc()
        .args(["list-conversions", png.to_str().unwrap(), "--json"])
        .output()
        .unwrap();
    assert!(output.status.success());

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["format"], "png");
    assert!(
        json["targets"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("jpg"))
    );
}

#[test]
fn convert_app_falls_back_without_running_app() {
    let dir = tempfile::tempdir().unwrap();
    let png = dir.path().join("pic.png");
    write_test_png(&png);

    // An isolated socket name nothing listens on: --app must convert locally.
    boinc()
        .env(
            "BOINC_SOCK",
            format!("boinc-test-none-{}.sock", std::process::id()),
        )
        .args(["convert", "--app", png.to_str().unwrap(), "--to", "jpg"])
        .assert()
        .success()
        .stdout(predicate::str::contains("pic.jpg"));

    assert!(dir.path().join("pic.jpg").is_file());
}

#[test]
fn convert_app_forwards_to_running_app() {
    use std::io::{BufRead, BufReader};

    let sock = format!("boinc-test-app-{}.sock", std::process::id());
    let listener = boinc_integration::ipc::bind_named(&sock).unwrap();
    let server = std::thread::spawn(move || {
        use boinc_integration::ipc::ListenerExt as _;
        let conn = listener.incoming().next().unwrap().unwrap();
        BufReader::new(conn)
            .lines()
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
    });

    let dir = tempfile::tempdir().unwrap();
    let png = dir.path().join("pic.png");
    write_test_png(&png);

    boinc()
        .env("BOINC_SOCK", &sock)
        .args(["convert", "--app", png.to_str().unwrap(), "--to", "jpg"])
        .assert()
        .success()
        .stdout(predicate::str::contains("queued in the Boinc app"));

    let lines = server.join().unwrap();
    assert_eq!(lines.len(), 1, "expected one IPC message: {lines:?}");
    let msg: serde_json::Value = serde_json::from_str(&lines[0]).unwrap();
    assert_eq!(msg["cmd"], "convert");
    assert_eq!(msg["to"], "jpg");
    assert!(
        msg["input"].as_str().unwrap().ends_with("pic.png"),
        "input should be the absolute source path: {msg}"
    );
    // The conversion was handed off, not run here.
    assert!(!dir.path().join("pic.jpg").exists());
}
