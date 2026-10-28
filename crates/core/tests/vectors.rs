use nextmoe_pow::{Alg, Challenge, Hasher, Scope, Token, verify};
use serde_json::Value;
use sha2::{Digest, Sha256};

fn vectors() -> Value {
    serde_json::from_str(include_str!("../../../testdata/vectors.json")).unwrap()
}

fn unhex(s: &str) -> Vec<u8> {
    (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect()
}

fn key(v: &Value, name: &str) -> Vec<u8> {
    unhex(v["keys"][name].as_str().unwrap())
}

#[test]
fn challenges_encode_and_sign() {
    let v = vectors();
    for c in v["challenges"].as_array().unwrap() {
        let alg = match c["alg"]["name"].as_str().unwrap() {
            "sha256" => Alg::Sha256,
            _ => Alg::Argon2id {
                memory_kib: c["alg"]["memory_kib"].as_u64().unwrap() as u32,
                iterations: c["alg"]["iterations"].as_u64().unwrap() as u8,
            },
        };
        let challenge = Challenge {
            alg,
            k: c["k"].as_u64().unwrap() as u8,
            difficulty: c["difficulty"].as_u64().unwrap() as u32,
            expires_at: c["expires_at"].as_u64().unwrap(),
            salt: unhex(c["salt"].as_str().unwrap()).try_into().unwrap(),
            scope: Scope { site: c["site"].as_str().unwrap().into(), action: c["action"].as_str().unwrap().into() },
        };
        let name = &c["name"];
        assert_eq!(challenge.encode().unwrap(), unhex(c["payload"].as_str().unwrap()), "{name}");
        let token = challenge.sign(&key(&v, c["key"].as_str().unwrap())).unwrap();
        assert_eq!(token, c["token"].as_str().unwrap(), "{name}");
        let parsed = Token::parse(&token).unwrap();
        assert_eq!(parsed.challenge, challenge, "{name}");
        assert_eq!(parsed.seed().to_vec(), unhex(c["seed"].as_str().unwrap()), "{name}");
    }
}

#[test]
fn hashes() {
    for h in vectors()["hashes"].as_array().unwrap() {
        let token = Token::parse(h["token"].as_str().unwrap()).unwrap();
        let nonce: u64 = h["nonce"].as_str().unwrap().parse().unwrap();
        let got = Hasher::new(&token).hash(h["sub"].as_u64().unwrap() as u8, nonce);
        assert_eq!(got.to_vec(), unhex(h["hash"].as_str().unwrap()), "{h}");
    }
}

#[test]
fn sha256_fast_path_matches_digest() {
    let token = Token::parse(vectors()["challenges"][0]["token"].as_str().unwrap()).unwrap();
    let mut hasher = Hasher::new(&token);
    for (sub, nonce) in [(0u8, 0u64), (3, 12345), (63, u64::MAX)] {
        let mut msg = token.seed().to_vec();
        msg.push(sub);
        msg.extend_from_slice(&nonce.to_be_bytes());
        assert_eq!(hasher.hash(sub, nonce), <[u8; 32]>::from(Sha256::digest(&msg)));
    }
}

#[test]
fn verify_cases() {
    let v = vectors();
    for case in v["verify"].as_array().unwrap() {
        let keys: Vec<Vec<u8>> =
            case["keys"].as_array().unwrap().iter().map(|k| key(&v, k.as_str().unwrap())).collect();
        let keys: Vec<&[u8]> = keys.iter().map(Vec::as_slice).collect();
        let scope =
            Scope { site: case["site"].as_str().unwrap().into(), action: case["action"].as_str().unwrap().into() };
        let result = verify(&keys, case["solution"].as_str().unwrap(), &scope, case["now"].as_u64().unwrap());
        let name = &case["name"];
        match result {
            Ok(ok) => {
                assert_eq!(case["expect"], "ok", "{name}");
                assert_eq!(case["replay_key"], ok.replay_key.as_str(), "{name}");
            }
            Err(e) => assert_eq!(case["expect"], e.code(), "{name}"),
        }
    }
}

#[test]
fn parallel_solutions_verify() {
    use nextmoe_pow::{decode_solution, encode_solution, solve_parallel};
    use std::sync::atomic::AtomicBool;
    use std::sync::{Arc, Mutex};

    let v = vectors();
    for case in v["verify"].as_array().unwrap().iter().filter(|c| c["name"].as_str().unwrap().ends_with("/ok")) {
        let (token, _) = decode_solution(case["solution"].as_str().unwrap()).unwrap();
        let reports = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&reports);
        let nonces = solve_parallel(&token, 4, &AtomicBool::new(false), move |p| sink.lock().unwrap().push(p)).unwrap();
        let scope =
            Scope { site: case["site"].as_str().unwrap().into(), action: case["action"].as_str().unwrap().into() };
        let solution = encode_solution(&token, &nonces);
        assert!(verify(&[&key(&v, "a")], &solution, &scope, case["now"].as_u64().unwrap()).is_ok(), "{}", case["name"]);
        let last = *reports.lock().unwrap().last().unwrap();
        assert_eq!(last.solved, token.challenge.k, "{}", case["name"]);
    }
}

#[test]
fn parallel_solve_can_be_cancelled() {
    use nextmoe_pow::solve_parallel;
    use std::sync::atomic::AtomicBool;

    let c = Challenge {
        alg: Alg::Sha256,
        k: 1,
        difficulty: u32::MAX,
        expires_at: 1,
        salt: [0; 16],
        scope: Scope { site: String::new(), action: String::new() },
    };
    let token = Token::parse(&c.sign(b"k").unwrap()).unwrap();
    let cancel = AtomicBool::new(false);
    let result = std::thread::scope(|s| {
        let handle = s.spawn(|| solve_parallel(&token, 2, &cancel, |_| {}));
        std::thread::sleep(std::time::Duration::from_millis(50));
        cancel.store(true, std::sync::atomic::Ordering::Relaxed);
        handle.join().unwrap()
    });
    assert_eq!(result, None);
}
