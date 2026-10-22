use assert_cmd::Command;
use predicates::str::contains;

#[test]
fn bench_smoke_awgn() {
    Command::cargo_bin("bench")
        .unwrap()
        .args(["--smoke", "--channel", "awgn", "--iterations", "2"])
        .assert()
        .success()
        .stdout(contains("LDPC Benchmark Harness"))
        .stdout(contains("Bit-Flip Benchmark"))
        .stdout(contains("SPA LLR Benchmark"));
}

#[test]
fn bench_smoke_rayleigh() {
    Command::cargo_bin("bench")
        .unwrap()
        .args(["--smoke", "--channel", "rayleigh"])
        .assert()
        .success();
}

#[test]
fn bench_smoke_rician() {
    Command::cargo_bin("bench")
        .unwrap()
        .args(["--smoke", "--channel", "rician"])
        .assert()
        .success();
}

#[test]
fn bench_smoke_nakagami() {
    Command::cargo_bin("bench")
        .unwrap()
        .args(["--smoke", "--channel", "nakagami"])
        .assert()
        .success();
}

#[test]
fn bench_custom_trials() {
    Command::cargo_bin("bench")
        .unwrap()
        .args(["--smoke", "--trials", "1"])
        .assert()
        .success()
        .stdout(contains("Trials: 1"));
}

#[test]
fn bench_custom_snr() {
    Command::cargo_bin("bench")
        .unwrap()
        .args(["--smoke", "--snr", "5.0"])
        .assert()
        .success()
        .stdout(contains("SNR: 5"));
}

#[test]
fn bench_custom_iterations() {
    Command::cargo_bin("bench")
        .unwrap()
        .args(["--smoke", "--iterations", "7"])
        .assert()
        .success()
        .stdout(contains("Max iterations/trial: 7"));
}

#[test]
fn bench_help() {
    Command::cargo_bin("bench")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(contains("LDPC Benchmark Harness"));
}

#[test]
fn bench_invalid_channel() {
    Command::cargo_bin("bench")
        .unwrap()
        .args(["--channel", "invalid"])
        .assert()
        .failure();
}
