use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering::Relaxed};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

use crate::challenge::{Alg, Token};
use crate::puzzle::Hasher;

const PROGRESS_EVERY: Duration = Duration::from_millis(50);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Progress {
    pub solved: u8,
    pub k: u8,
    pub attempts: u64,
    pub expected_attempts: u64,
}

/// Solves on `threads` OS threads. Every thread works on some unsolved sub-puzzle, taking
/// disjoint nonce batches from it, so threads can share one sub-puzzle without wasted work.
/// Returns `None` when `cancel` is set before every sub-puzzle is solved.
pub fn solve_parallel(
    token: &Token,
    threads: usize,
    cancel: &AtomicBool,
    on_progress: impl Fn(Progress) + Sync,
) -> Option<Vec<u64>> {
    let challenge = &token.challenge;
    let k = usize::from(challenge.k);
    let batch = match challenge.alg {
        Alg::Sha256 => 1 << 14,
        Alg::Argon2id { .. } => 1,
    };
    let next: Vec<AtomicU64> = (0..k).map(|_| AtomicU64::new(0)).collect();
    let found: Vec<OnceLock<u64>> = (0..k).map(|_| OnceLock::new()).collect();
    let solved = AtomicUsize::new(0);
    let attempts = AtomicU64::new(0);
    let last_report = Mutex::new(Instant::now());
    let expected_attempts = challenge.k as u64 * u64::from(challenge.difficulty);
    let report = |solved_one: bool| {
        if let Ok(mut last) = last_report.try_lock()
            && (solved_one || last.elapsed() >= PROGRESS_EVERY)
        {
            *last = Instant::now();
            let solved = solved.load(Relaxed) as u8;
            on_progress(Progress { solved, k: challenge.k, attempts: attempts.load(Relaxed), expected_attempts });
        }
    };

    thread::scope(|scope| {
        for worker in 0..threads.max(1) {
            let (next, found, solved, attempts, report) = (&next, &found, &solved, &attempts, &report);
            scope.spawn(move || {
                let mut hasher = Hasher::new(token);
                let mut sub = worker % k;
                while !cancel.load(Relaxed) {
                    let Some(i) = (0..k).map(|j| (sub + j) % k).find(|&i| found[i].get().is_none()) else {
                        break;
                    };
                    sub = i;
                    let start = next[i].fetch_add(batch, Relaxed);
                    let hit = hasher.search(i as u8, start, batch);
                    attempts.fetch_add(hit.map_or(batch, |n| n - start + 1), Relaxed);
                    let newly_solved = hit.is_some_and(|n| found[i].set(n).is_ok());
                    if newly_solved {
                        solved.fetch_add(1, Relaxed);
                    }
                    report(newly_solved);
                }
            });
        }
    });

    if cancel.load(Relaxed) {
        return None;
    }
    on_progress(Progress { solved: challenge.k, k: challenge.k, attempts: attempts.load(Relaxed), expected_attempts });
    Some(found.into_iter().map(|f| f.into_inner().expect("loop exits only when solved or cancelled")).collect())
}
