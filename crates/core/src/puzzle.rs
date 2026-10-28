use argon2::{Algorithm, Argon2, Block, Params, Version};
use sha2::block_api::compress256;

use crate::challenge::{Alg, Token};

const SHA256_IV: [u32; 8] =
    [0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19];
const SUB_AT: usize = 32;
const NONCE_AT: usize = 33;
const MSG_LEN: usize = 41;

pub fn threshold(difficulty: u32) -> u64 {
    u64::MAX / u64::from(difficulty.max(1))
}

pub fn meets(hash: &[u8; 32], difficulty: u32) -> bool {
    u64::from_be_bytes(hash[..8].try_into().unwrap()) <= threshold(difficulty)
}

/// Evaluates sub-puzzle hashes for one challenge, reusing its buffers
/// (the SHA-256 block, or the Argon2 memory) across attempts.
pub struct Hasher {
    seed: [u8; 32],
    difficulty: u32,
    kind: Kind,
}

enum Kind {
    Sha256 { block: [u8; 64] },
    Argon2id { ctx: Argon2<'static>, memory: Vec<Block> },
}

impl Hasher {
    pub fn new(token: &Token) -> Self {
        let challenge = &token.challenge;
        let seed = token.seed();
        let kind = match challenge.alg {
            Alg::Sha256 => {
                let mut block = [0u8; 64];
                block[..32].copy_from_slice(&seed);
                block[MSG_LEN] = 0x80;
                block[56..].copy_from_slice(&((MSG_LEN as u64) * 8).to_be_bytes());
                Kind::Sha256 { block }
            }
            Alg::Argon2id { memory_kib, iterations } => {
                let params = Params::new(memory_kib, u32::from(iterations), 1, Some(32))
                    .expect("Challenge::decode bounds the Argon2 parameters");
                let memory = vec![Block::default(); params.block_count()];
                Kind::Argon2id { ctx: Argon2::new(Algorithm::Argon2id, Version::V0x13, params), memory }
            }
        };
        Hasher { seed, difficulty: challenge.difficulty, kind }
    }

    pub fn hash(&mut self, sub: u8, nonce: u64) -> [u8; 32] {
        match &mut self.kind {
            Kind::Sha256 { block } => {
                block[SUB_AT] = sub;
                block[NONCE_AT..MSG_LEN].copy_from_slice(&nonce.to_be_bytes());
                let mut state = SHA256_IV;
                compress256(&mut state, std::slice::from_ref(block));
                let mut out = [0u8; 32];
                for (chunk, word) in out.chunks_exact_mut(4).zip(state) {
                    chunk.copy_from_slice(&word.to_be_bytes());
                }
                out
            }
            Kind::Argon2id { ctx, memory } => {
                let mut salt = [0u8; 33];
                salt[..32].copy_from_slice(&self.seed);
                salt[32] = sub;
                let mut out = [0u8; 32];
                ctx.hash_password_into_with_memory(&nonce.to_be_bytes(), &salt, &mut out, memory.as_mut_slice())
                    .expect("salt and output lengths are fixed and valid");
                out
            }
        }
    }

    pub fn check(&mut self, sub: u8, nonce: u64) -> bool {
        let difficulty = self.difficulty;
        meets(&self.hash(sub, nonce), difficulty)
    }

    /// Tries nonces `start..start + count` for one sub-puzzle and returns the first that passes.
    pub fn search(&mut self, sub: u8, start: u64, count: u64) -> Option<u64> {
        (start..start.saturating_add(count)).find(|&nonce| self.check(sub, nonce))
    }
}
