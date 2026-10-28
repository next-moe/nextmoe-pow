//! Regenerates testdata/vectors.json: `cargo run -p nextmoe-pow --example vectors > testdata/vectors.json`.

use base64ct::Encoding as _;
use nextmoe_pow::{Alg, Challenge, Hasher, Scope, Token, encode_solution, solve, verify};
use serde_json::{Value, json};

const KEY_A: &[u8] = b"nextmoe-pow test key A, 32 bytes";
const KEY_B: &[u8] = b"nextmoe-pow test key B, 32 bytes";
const NOW: u64 = 1_800_000_000;

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn scope(site: &str, action: &str) -> Scope {
    Scope { site: site.into(), action: action.into() }
}

fn challenge(alg: Alg, k: u8, difficulty: u32, salt_byte: u8, scope: Scope) -> Challenge {
    Challenge { alg, k, difficulty, expires_at: NOW + 300, salt: [salt_byte; 16], scope }
}

fn alg_json(alg: Alg) -> Value {
    match alg {
        Alg::Sha256 => json!({ "name": "sha256" }),
        Alg::Argon2id { memory_kib, iterations } => {
            json!({ "name": "argon2id", "memory_kib": memory_kib, "iterations": iterations })
        }
    }
}

fn main() {
    let search = scope("www.kungal.com", "agent.search");
    let cases = [
        ("sha256", challenge(Alg::Sha256, 4, 1000, 0x11, search.clone())),
        ("sha256-k1", challenge(Alg::Sha256, 1, 50, 0x12, scope("", ""))),
        ("argon2id-64k", challenge(Alg::Argon2id { memory_kib: 64, iterations: 1 }, 3, 8, 0x21, search.clone())),
        (
            "argon2id-1m-t2",
            challenge(Alg::Argon2id { memory_kib: 1024, iterations: 2 }, 2, 2, 0x22, scope("ゲーム.example", "下载")),
        ),
    ];

    let mut challenges = Vec::new();
    let mut hashes = Vec::new();
    let mut solutions = Vec::new();
    for (name, c) in &cases {
        let token_str = c.sign(KEY_A).unwrap();
        let token = Token::parse(&token_str).unwrap();
        challenges.push(json!({
            "name": name,
            "key": "a",
            "alg": alg_json(c.alg),
            "k": c.k,
            "difficulty": c.difficulty,
            "expires_at": c.expires_at,
            "salt": hex(&c.salt),
            "site": c.scope.site,
            "action": c.scope.action,
            "payload": hex(&c.encode().unwrap()),
            "token": token_str,
            "seed": hex(&token.seed()),
        }));
        let mut hasher = Hasher::new(&token);
        for (sub, nonce) in [(0u8, 0u64), (c.k - 1, 1), (0, u64::MAX)] {
            hashes.push(json!({ "token": token_str, "sub": sub, "nonce": nonce.to_string(), "hash": hex(&hasher.hash(sub, nonce)) }));
        }
        let nonces = solve(&token);
        solutions.push((name, token, nonces, c.scope.clone()));
    }

    let mut verify_cases = Vec::new();
    let mut case = |name: String, keys: &[&str], solution: String, scope: &Scope, now: u64| {
        let key_bytes: Vec<&[u8]> = keys.iter().map(|k| if *k == "a" { KEY_A } else { KEY_B }).collect();
        let result = verify(&key_bytes, &solution, scope, now);
        let mut v = json!({
            "name": name,
            "keys": keys,
            "solution": solution,
            "site": scope.site,
            "action": scope.action,
            "now": now,
            "expect": result.as_ref().map(|_| "ok").unwrap_or_else(|e| e.code()),
        });
        if let Ok(ok) = result {
            v["replay_key"] = json!(ok.replay_key);
        }
        verify_cases.push(v);
    };

    for (name, token, nonces, scope) in &solutions {
        case(format!("{name}/ok"), &["a"], encode_solution(token, nonces), scope, NOW);
    }
    let (_, token, nonces, sc) = &solutions[0];
    let good = encode_solution(token, nonces);
    case("rotated-key/ok".into(), &["b", "a"], good.clone(), sc, NOW);
    case("wrong-key".into(), &["b"], good.clone(), sc, NOW);
    case("last-valid-second".into(), &["a"], good.clone(), sc, token.challenge.expires_at - 1);
    case("expired".into(), &["a"], good.clone(), sc, token.challenge.expires_at);
    case("site-mismatch".into(), &["a"], good.clone(), &scope("www.kungal.org", &sc.action), NOW);
    case("action-mismatch".into(), &["a"], good.clone(), &scope(&sc.site, "agent.chat"), NOW);
    case("too-few-nonces".into(), &["a"], encode_solution(token, &nonces[1..]), sc, NOW);
    case("too-many-nonces".into(), &["a"], encode_solution(token, &[nonces.as_slice(), &[0]].concat()), sc, NOW);
    let mut hasher = Hasher::new(token);
    let bad = (0..).find(|&n| !hasher.check(2, n)).unwrap();
    let mut wrong = nonces.clone();
    wrong[2] = bad;
    case("invalid-nonce".into(), &["a"], encode_solution(token, &wrong), sc, NOW);

    let tampered = {
        let mut c = token.challenge.clone();
        c.difficulty = 1;
        let forged = Token::parse(&c.sign(KEY_B).unwrap()).unwrap();
        let mac = token.encode().split_once('.').unwrap().1.to_string();
        let payload = forged.encode().split_once('.').unwrap().0.to_string();
        format!("{payload}.{mac}.{}", good.rsplit_once('.').unwrap().1)
    };
    case("tampered-payload".into(), &["a"], tampered, sc, NOW);
    case("garbage".into(), &["a"], "not a solution".into(), sc, NOW);
    case("missing-nonces".into(), &["a"], token.encode(), sc, NOW);
    case("trailing-segment".into(), &["a"], format!("{good}.AAAAAAAAAAA"), sc, NOW);
    let non_canonical = {
        const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
        let last = *good.as_bytes().last().unwrap();
        let value = ALPHABET.iter().position(|&c| c == last).unwrap();
        format!("{}{}", &good[..good.len() - 1], ALPHABET[value | 1] as char)
    };
    case("non-canonical-base64".into(), &["a"], non_canonical, sc, NOW);
    let bad_version = {
        let (payload, rest) = good.split_once('.').unwrap();
        let mut bytes = base64ct::Base64UrlUnpadded::decode_vec(payload).unwrap();
        bytes[0] = 2;
        format!("{}.{rest}", base64ct::Base64UrlUnpadded::encode_string(&bytes))
    };
    let (head, tail) = good.split_at(10);
    case("newline-in-token".into(), &["a"], format!("{head}\n{tail}"), sc, NOW);
    case("unknown-version".into(), &["a"], bad_version, sc, NOW);

    let out = json!({
        "comment": "Generated by crates/core/examples/vectors.rs. Rust, Go and TS must all pass these.",
        "keys": { "a": hex(KEY_A), "b": hex(KEY_B) },
        "challenges": challenges,
        "hashes": hashes,
        "verify": verify_cases,
    });
    println!("{}", serde_json::to_string_pretty(&out).unwrap());
}
