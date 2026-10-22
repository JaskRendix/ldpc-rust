use assert_cmd::Command;
use predicates::str::contains;

#[test]
fn custom_ber_spa_help() {
    Command::cargo_bin("custom_ber_spa")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(contains("Custom Matrix"));
}

#[test]
fn custom_ber_spa_smoke_default() {
    Command::cargo_bin("custom_ber_spa")
        .unwrap()
        .arg("--smoke")
        .assert()
        .success()
        .stdout(contains("snr_db,ber,trials,error_bits,total_bits"));
}

#[test]
fn custom_ber_spa_custom_args() {
    Command::cargo_bin("custom_ber_spa")
        .unwrap()
        .args([
            "--smoke",
            "--seed",
            "123",
            "--min-error-bits",
            "5",
            "--max-trials",
            "10",
            "--iterations",
            "3",
        ])
        .assert()
        .success();
}

#[test]
fn custom_ber_spa_rayleigh() {
    Command::cargo_bin("custom_ber_spa")
        .unwrap()
        .args(["--smoke", "--channel", "rayleigh"])
        .assert()
        .success();
}

#[test]
fn custom_ber_spa_rician() {
    Command::cargo_bin("custom_ber_spa")
        .unwrap()
        .args(["--smoke", "--channel", "rician"])
        .assert()
        .success();
}

#[test]
fn custom_ber_spa_nakagami() {
    Command::cargo_bin("custom_ber_spa")
        .unwrap()
        .args(["--smoke", "--channel", "nakagami"])
        .assert()
        .success();
}

#[test]
fn custom_ber_spa_flooding() {
    Command::cargo_bin("custom_ber_spa")
        .unwrap()
        .args(["--smoke", "--scheduling", "flooding"])
        .assert()
        .success();
}

#[test]
fn custom_ber_spa_layered() {
    Command::cargo_bin("custom_ber_spa")
        .unwrap()
        .args(["--smoke", "--scheduling", "layered"])
        .assert()
        .success();
}

#[test]
fn custom_ber_spa_invalid_channel() {
    Command::cargo_bin("custom_ber_spa")
        .unwrap()
        .args(["--channel", "bad"])
        .assert()
        .failure();
}

#[test]
fn custom_ber_spa_invalid_scheduling() {
    Command::cargo_bin("custom_ber_spa")
        .unwrap()
        .args(["--scheduling", "bad"])
        .assert()
        .failure();
}
