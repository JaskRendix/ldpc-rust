use assert_cmd::Command;
use predicates::str::contains;

#[test]
fn ber_spa_help() {
    Command::cargo_bin("ber_spa")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(contains("LDPC SPA Bit Error Rate"));
}

#[test]
fn ber_spa_smoke_default() {
    Command::cargo_bin("ber_spa")
        .unwrap()
        .arg("--smoke")
        .assert()
        .success()
        .stdout(contains("snr_db,ber,trials,error_bits,total_bits"));
}

#[test]
fn ber_spa_smoke_rayleigh() {
    Command::cargo_bin("ber_spa")
        .unwrap()
        .args(["--smoke", "--channel", "rayleigh"])
        .assert()
        .success();
}

#[test]
fn ber_spa_smoke_rician() {
    Command::cargo_bin("ber_spa")
        .unwrap()
        .args(["--smoke", "--channel", "rician"])
        .assert()
        .success();
}

#[test]
fn ber_spa_smoke_nakagami() {
    Command::cargo_bin("ber_spa")
        .unwrap()
        .args(["--smoke", "--channel", "nakagami"])
        .assert()
        .success();
}

#[test]
fn ber_spa_flooding() {
    Command::cargo_bin("ber_spa")
        .unwrap()
        .args(["--smoke", "--scheduling", "flooding"])
        .assert()
        .success();
}

#[test]
fn ber_spa_layered() {
    Command::cargo_bin("ber_spa")
        .unwrap()
        .args(["--smoke", "--scheduling", "layered"])
        .assert()
        .success();
}

#[test]
fn ber_spa_custom_args() {
    Command::cargo_bin("ber_spa")
        .unwrap()
        .args([
            "--smoke",
            "--seed",
            "123",
            "--min-error-bits",
            "5",
            "--max-trials",
            "10",
        ])
        .assert()
        .success();
}

#[test]
fn ber_spa_invalid_channel() {
    Command::cargo_bin("ber_spa")
        .unwrap()
        .args(["--channel", "invalid"])
        .assert()
        .failure();
}

#[test]
fn ber_spa_invalid_scheduling() {
    Command::cargo_bin("ber_spa")
        .unwrap()
        .args(["--scheduling", "invalid"])
        .assert()
        .failure();
}

#[test]
fn ber_spa_csv_header_present() {
    Command::cargo_bin("ber_spa")
        .unwrap()
        .arg("--smoke")
        .assert()
        .success()
        .stdout(contains("snr_db,ber,trials,error_bits,total_bits"))
        .stdout(contains("0,"));
}
