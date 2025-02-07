// File: benches/random.rs
// Project: Bifrost
// Creation date: Monday 27 January 2025
// Author: Vincent Berthier <vincent.berthier@posteo.org>
// -----
// Last modified: Friday 07 February 2025 @ 15:33:43
// Modified by: Vincent Berthier
// -----
// Copyright (c) 2025 <Vincent Berthier>
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the 'Software'), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED 'AS IS', WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

#![expect(clippy::unwrap_used)]

use std::sync::{Mutex, OnceLock};

use criterion::{criterion_group, criterion_main, Criterion};
use ed25519_dalek::SigningKey;
use rand::{rngs::OsRng, SeedableRng};
use rand_chacha::ChaCha20Rng;

static RNG: OnceLock<Mutex<ChaCha20Rng>> = OnceLock::new();

fn draw_osrns() -> SigningKey {
    let mut rng = OsRng;
    SigningKey::generate(&mut rng)
}

fn draw_chacha20_entropy() -> SigningKey {
    let mut rng = ChaCha20Rng::from_entropy();
    SigningKey::generate(&mut rng)
}

fn draw_chacha20_seed() -> SigningKey {
    let mut rng = ChaCha20Rng::seed_from_u64(0);
    SigningKey::generate(&mut rng)
}

fn draw_with_static() -> SigningKey {
    let mut rng = RNG.get_or_init(init_rand_engine).lock().unwrap();
    SigningKey::generate(&mut *rng)
}

fn init_rand_engine() -> Mutex<ChaCha20Rng> {
    let seed = 0_u64;
    let rng = ChaCha20Rng::seed_from_u64(seed);

    Mutex::new(rng)
}

pub fn random_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("Random Engine");
    group.bench_function("OsRng", |b| {
        b.iter(draw_osrns);
    });
    group.bench_function("ChaCha20 (seed)", |b| {
        b.iter(draw_chacha20_seed);
    });
    group.bench_function("ChaCha20 (entropy)", |b| {
        b.iter(draw_chacha20_entropy);
    });
    group.bench_function("ChaCha20 (with static)", |b| {
        b.iter(draw_with_static);
    });
}

criterion_group!(benches, random_benchmark);
criterion_main!(benches);
