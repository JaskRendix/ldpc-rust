use criterion::{Criterion, criterion_group, criterion_main};
use ldpc_rust::bitarray::BitArray;
use ldpc_rust::channel::bpsk_awgn_llr;
use ldpc_rust::ldpc_decoder::LdpcDecoder;
use ldpc_rust::matrices::h_256_512::H_256_512;
use ldpc_rust::spa_decoder_llr::SpaDecoderLLR;
use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};

fn bench_bitflip_trial(c: &mut Criterion) {
    let decoder = LdpcDecoder::new(&H_256_512);

    c.bench_function("bitflip_full_trial_3_errors", |b| {
        b.iter(|| {
            let mut rng = StdRng::seed_from_u64(42);
            let mut cw = [0u8; 64];
            let mut flipped = std::collections::HashSet::new();

            while flipped.len() < 3 {
                let idx = rng.random_range(0..512);
                if flipped.insert(idx) {
                    BitArray::xor_bit(&mut cw, idx);
                }
            }

            for _ in 0..20 {
                if decoder.iterate_bitflip(&mut cw) {
                    break;
                }
            }
        });
    });
}

fn bench_spa_decode(c: &mut Criterion) {
    let mut decoder = SpaDecoderLLR::new(&H_256_512);
    decoder.set_max_iter(20);
    let mut rng = StdRng::seed_from_u64(42);

    let n = 512;
    let cw = vec![0u8; n];
    let mut llr = vec![0.0f64; n];
    for i in 0..n {
        llr[i] = bpsk_awgn_llr(cw[i], 0.5, &mut rng);
    }

    c.bench_function("spa_llr_decode_512", |b| {
        b.iter(|| {
            let _ = decoder.decode(&llr);
        });
    });
}

criterion_group!(benches, bench_bitflip_trial, bench_spa_decode);
criterion_main!(benches);
