# LDPC Rust

A Rust implementation of CCSDS LDPC decoding algorithms originally prototyped in the DelfiSpace *LDPC‑Simulation* project.

The codebase replaces the original Python/C++ scripts with a single memory‑safe, deterministic Rust library.

All decoding operations use fixed‑size arrays, checked indexing, and embedded CCSDS parity‑check matrices.

The implementation covers:

* **Hard-decision decoders:** Gallager‑A, Gallager‑B, WBF, MWBF, NWBF
* **Soft-decision decoders:** SPA, Min‑Sum, and Normalized Min‑Sum (NMS) in the LLR domain, featuring SIMD-accelerated vectorization for inner loops
* **Optimized SPA Architecture:** Pre-allocated row buffers in `SpaDecoderLLR::decode` to eliminate per-iteration heap allocations
* **Const-Generic Matrix Sizes:** Generic `const M: usize, const N: usize` implementations across encoders and decoders supporting alternative CCSDS matrices (128×256 and 256×512) without duplicated logic
* **Custom Parity-Check Matrix Macro:** Ergonomic `define_custom_matrix!` macro helper allowing researchers to define and test arbitrary $(M, N)$ block codes using the const-generic architecture without modifying library source files
* **Robust Convergence Metrics:** Structured decode return types exposing iteration counts and convergence status for microservice health tracking
* **Systematic LDPC Encoder:** Generates valid codewords ($k = 256 \to n = 512$) via lazily computed generator matrices over $\text{GF}(2)$
* **Embedded Flight-Software Readiness (`no_std`):** Core decoders and encoders support `#![no_std]` with `alloc`, allowing bare-metal execution on microcontrollers or flight computers without an operating system
* **Zero-Copy Axum Routing:** JSON request handlers mapping directly to fixed-size arrays and slices via Serde, backed by structured telemetry via `tracing`
* **Flexible CLI Arguments:** Runtime configuration for simulation scripts via `clap` (e.g., trial limits, seeds, smoke modes)
* **BER Simulation Tools & Performance Benchmarks:** Custom multithreaded simulation binaries, CSV outputs, and statistical Criterion suites
* **Comprehensive Test Suite:** Includes property-based testing via `proptest` alongside deterministic correctness and fuzz trials

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

## Embedded Flight-Software Readiness (`no_std`)

The core library is fully compatible with `#![no_std]` (using `alloc`), allowing deterministic decoders and encoders to run directly on bare-metal microcontrollers or space-grade flight computers without an operating system.

When compiled in standard environments, the default `std` feature flag automatically unlocks the Axum web service, CLI tools, and tracing infrastructure.

---

## Project Layout

```
src/
bitarray.rs
channel.rs
encoder.rs
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
custom_ber_spa.rs
server.rs

benches/
ldpc_bench.rs

tests/
custom_matrix_tests.rs
encoder_tests.rs
fuzz_decoders.rs
ldpc_tests.rs
property_tests.rs
server_tests.rs
spa_decoder_tests.rs

Cargo.toml
README.md
```

---

## Systematic LDPC Encoder

The library includes an embedded systematic encoder for the CCSDS $(512, 256)$ code that maps raw message bits ($k = 256$) into valid systematic codewords ($n = 512$) using a lazily computed generator matrix over $\text{GF}(2)$.

```rust
use ldpc_rust::encoder::LDPC_ENCODER;

let message = [0u8; 256]; // raw message
let codeword = LDPC_ENCODER.encode(&message); // 512-bit systematic codeword [u | p]
```

---

## Custom Matrices via Macro

Define and evaluate arbitrary block codes seamlessly using the built-in macro interface:

```rust
use ldpc_rust::define_custom_matrix;
use ldpc_rust::spa_decoder_llr::SpaDecoderLLR;

define_custom_matrix!(
    pub struct CustomMatrix4x8,
    const M = 4,
    const N = 8,
    [
        [1, 1, 0, 0, 1, 0, 0, 0],
        [0, 1, 1, 0, 0, 1, 0, 0],
        [0, 0, 1, 1, 0, 0, 1, 0],
        [1, 0, 0, 1, 0, 0, 0, 1],
    ]
);

let mut decoder: SpaDecoderLLR<{ CustomMatrix4x8::ROWS }, { CustomMatrix4x8::COLS }> =
    SpaDecoderLLR::new(&CustomMatrix4x8::DATA);
```

---

## Running Tests

Run standard tests (including the web server and CLI utilities):

```bash
cargo test
```

Run bare-metal/embedded tests (`no_std` mode):

```bash
cargo test --no-default-features
```

Property-based testing suites automatically fuzz encoder-decoder roundtrips across randomized message payloads and noise bursts.

---

## BER Simulation

The multithreaded SPA/Min‑Sum decoder generates BER curves concurrently across multiple SNR points. Live progress indicators are printed to `stderr` during execution, keeping `stdout` clean for CSV redirection.

```bash
cargo run --release --bin ber_spa -- --seed 42 > ber_spa_256_512.csv
```

For custom matrix simulations:

```bash
cargo run --release --bin custom_ber_spa > custom_ber_8_16.csv
```

---

## Benchmarks

### Custom Binary Benchmarks

```bash
cargo run --release --bin bench
```

Reports total time, average time per trial, and throughput for custom micro-benchmarks.

### Criterion Statistical Suite

```bash
cargo bench
```

Executes statistical performance tracking for bit-flip trials and SPA LLR decoding loops, outputting detailed distribution metrics.

---

## Axum Microservice

An HTTP service exposes the decoders for external tools with structured request telemetry.

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

---

## Reference

This project is based on CCSDS LDPC decoding algorithms and the DelfiSpace *LDPC‑Simulation* project:

[https://github.com/DelfiSpace/LDPC-Simulation](https://github.com/DelfiSpace/LDPC-Simulation)

The Rust version removes pointer‑level edge cases and undefined behavior present in the C++ implementation while maintaining algorithmic structure and matrix definitions consistent with CCSDS specifications.
