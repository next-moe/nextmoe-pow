use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::available_parallelism;
use std::time::Instant;

use flutter_rust_bridge::frb;
use nextmoe_pow::{Alg, Progress, Token, encode_solution, solve_parallel};

use crate::frb_generated::StreamSink;

pub struct ChallengeInfo {
    pub alg: String,
    pub k: u8,
    pub difficulty: u32,
    pub memory_kib: u32,
    pub iterations: u8,
    pub expires_at: i64,
    pub site: String,
    pub action: String,
}

/// A progress report; the last one carries `solution`.
pub struct SolveEvent {
    pub solved: u8,
    pub k: u8,
    pub attempts: i64,
    pub expected_attempts: i64,
    pub elapsed_ms: i64,
    pub expires_at: i64,
    pub solution: Option<String>,
}

fn parse(token: &str) -> Result<Token, String> {
    Token::parse(token).map_err(|e| e.code().to_string())
}

#[frb(sync)]
pub fn challenge_info(token: String) -> Result<ChallengeInfo, String> {
    let c = parse(&token)?.challenge;
    let (alg, memory_kib, iterations) = match c.alg {
        Alg::Sha256 => ("sha256", 0, 0),
        Alg::Argon2id { memory_kib, iterations } => ("argon2id", memory_kib, iterations),
    };
    Ok(ChallengeInfo {
        alg: alg.into(),
        k: c.k,
        difficulty: c.difficulty,
        memory_kib,
        iterations,
        expires_at: c.expires_at as i64,
        site: c.scope.site,
        action: c.scope.action,
    })
}

/// Solves on `threads` native threads (0 = one per core). Cancelling the Dart
/// subscription stops the solve at its next progress report.
pub fn solve(token: String, threads: u32, sink: StreamSink<SolveEvent>) -> Result<(), String> {
    let token = parse(&token)?;
    let threads = match threads {
        0 => available_parallelism().map_or(4, usize::from),
        n => n as usize,
    };
    let started = Instant::now();
    let cancel = AtomicBool::new(false);
    let event = |p: Progress, solution: Option<String>| SolveEvent {
        solved: p.solved,
        k: p.k,
        attempts: p.attempts as i64,
        expected_attempts: p.expected_attempts as i64,
        elapsed_ms: started.elapsed().as_millis() as i64,
        expires_at: token.challenge.expires_at as i64,
        solution,
    };
    let last = Mutex::new(None);
    let nonces = solve_parallel(&token, threads, &cancel, |p| {
        *last.lock().unwrap() = Some(p);
        if sink.add(event(p, None)).is_err() {
            cancel.store(true, Ordering::Relaxed);
        }
    });
    if let (Some(nonces), Some(p)) = (nonces, last.into_inner().unwrap()) {
        let _ = sink.add(event(p, Some(encode_solution(&token, &nonces))));
    }
    Ok(())
}
