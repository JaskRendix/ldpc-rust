# LDPC Rust

A Rust implementation of CCSDS LDPC decoding algorithms originally prototyped in the DelfiSpace *LDPC‑Simulation* project.

The codebase replaces the original Python/C++ scripts with a single memory‑safe, deterministic Rust library.

All decoding operations use fixed‑size arrays, checked indexing, and embedded CCSDS parity‑check matrices.

The implementation covers:

* hard‑decision decoders: Gallager‑A, Gallager‑B, WBF, MWBF, NWBF
* soft‑decision decoders: SPA, Min‑Sum, and Normalized Min‑Sum (NMS) in the LLR domain
* CCSDS matrices: 128×256 and 256×512
* BER simulation tools
* an Axum HTTP microservice
* benchmarks for hard‑ and soft‑decision decoders
* a test suite for correctness, safety, and randomized trials

The structure of the CCSDS reference algorithms is preserved.

Hard‑decision decoders behave as defined in the literature; Gallager‑B, MWBF, NWBF, and SPA provide reliable correction behavior across all bit positions.

WBF is included for completeness but does not guarantee convergence for every single‑bit error on the CCSDS matrices.

---

## Context: Why Rust

The original DelfiSpace repository mixes Python control logic with C++ decoding kernels.

Porting the algorithms to Rust consolidates the implementation into a single, safe binary and removes:

* Python loop overhead
* C++ pointer arithmetic
* manual memory management
* ad‑hoc threading scripts

Rust provides deterministic memory safety and predictable performance for LDPC decoding workloads that evaluate thousands of parity‑check equations per iteration.

The SPA, Min‑Sum, and NMS decoders run tight numerical loops without garbage‑collection pauses or undefined behavior.

---

## Project Layout

```
src/
  bitarray.rs
  ldpc_decoder.rs
  spa_decoder_llr.rs
  matrices/
    h_128_256.rs
    h_256_512.rs
    mod.rs
  server_router.rs

src/bin/
  ber_spa.rs
  bench.rs
  server.rs

tests/
  ldpc_tests.rs
  server_tests.rs
  spa_decoder_tests.rs

Cargo.toml
README.md

```

---

## Running Tests

```bash
cargo test

```

---

## BER Simulation

The SPA/Min‑Sum decoder generates BER curves for the CCSDS 256×512 code.

Output format is `(snr_db, ber)` pairs in CSV format.

```bash
cargo run --release --bin ber_spa > ber_spa_256_512.csv

```

---

## Benchmarks

```bash
cargo run --release --bin bench

```

Reports total time, average time per trial, and throughput.

---

## Axum Microservice

An HTTP service exposes the decoders for external tools.

Start:

```bash
cargo run --bin server

```

Health check:

```bash
curl http://localhost:8080/health

```

### Bit‑Flip Decode

```bash
curl -X POST http://localhost:8080/decode/bitflip \
     -H "Content-Type: application/json" \
     -d '{"cw":[...], "iterations":10}'

```

### SPA Decode

```bash
curl -X POST http://localhost:8080/decode/spa \
     -H "Content-Type: application/json" \
     -d '{"cw":[...], "snr_db":1.0, "iterations":10, "scaling_factor":0.75}'

```

---

## Docker

Build:

```bash
docker build -t ldpc-server .

```

Run:

```bash
docker run -p 8080:8080 ldpc-server

```

This setup uses only the Dockerfile.

Prometheus, Grafana, and docker‑compose are excluded.

---

## Reference

This project is based on CCSDS LDPC decoding algorithms and the DelfiSpace *LDPC‑Simulation* project:

[https://github.com/DelfiSpace/LDPC-Simulation](https://github.com/DelfiSpace/LDPC-Simulation)

The Rust version removes pointer‑level edge cases and undefined behavior present in the C++ implementation while maintaining algorithmic structure and matrix definitions consistent with CCSDS specifications.
