use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use nextmoe_pow::{Alg, Challenge, Hasher, Scope, Token};
use std::hint::black_box;

fn token(alg: Alg) -> Token {
    let scope = Scope { site: "www.kungal.com".into(), action: "agent.search".into() };
    let c = Challenge { alg, k: 1, difficulty: 1, expires_at: 4_000_000_000, salt: [7; 16], scope };
    Token::parse(&c.sign(b"bench").unwrap()).unwrap()
}

fn hash(c: &mut Criterion) {
    let mut group = c.benchmark_group("hash");
    let mut sha = Hasher::new(&token(Alg::Sha256));
    let mut nonce = 0u64;
    group.bench_function("sha256", |b| {
        b.iter(|| {
            nonce += 1;
            black_box(sha.hash(0, nonce))
        })
    });
    for memory_kib in [256, 512, 1024, 2048, 4096] {
        let mut argon = Hasher::new(&token(Alg::Argon2id { memory_kib, iterations: 1 }));
        group.bench_with_input(BenchmarkId::new("argon2id-t1", memory_kib), &memory_kib, |b, _| {
            b.iter(|| {
                nonce += 1;
                black_box(argon.hash(0, nonce))
            })
        });
    }
    group.finish();
}

criterion_group!(benches, hash);
criterion_main!(benches);
