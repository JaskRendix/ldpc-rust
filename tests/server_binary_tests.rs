use assert_cmd::Command;
use predicates::str::contains;

#[test]
fn server_help() {
    Command::cargo_bin("server")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(contains("LDPC Axum Microservice Server"));
}

#[test]
fn server_version() {
    Command::cargo_bin("server")
        .unwrap()
        .arg("--version")
        .assert()
        .success();
}

#[test]
fn server_invalid_host() {
    Command::cargo_bin("server")
        .unwrap()
        .args(["--host", "not_a_valid_address:::"])
        .assert()
        .failure();
}

#[test]
fn server_invalid_port() {
    Command::cargo_bin("server")
        .unwrap()
        .args(["--port", "70000"])
        .assert()
        .failure();
}
