mod challenge;
mod puzzle;

use base64ct::{Base64UrlUnpadded, Encoding};

pub use challenge::{Alg, Challenge, MAX_K, SALT_LEN, Scope, Token, VERSION};
pub use puzzle::{Hasher, meets, threshold};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Malformed,
    BadSignature,
    Expired,
    ScopeMismatch,
    WrongNonceCount,
    InvalidNonce,
}

impl Error {
    pub fn code(self) -> &'static str {
        match self {
            Error::Malformed => "malformed",
            Error::BadSignature => "bad_signature",
            Error::Expired => "expired",
            Error::ScopeMismatch => "scope_mismatch",
            Error::WrongNonceCount => "wrong_nonce_count",
            Error::InvalidNonce => "invalid_nonce",
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}

impl std::error::Error for Error {}

pub fn encode_solution(token: &Token, nonces: &[u64]) -> String {
    let bytes: Vec<u8> = nonces.iter().flat_map(|n| n.to_be_bytes()).collect();
    format!("{}.{}", token.encode(), Base64UrlUnpadded::encode_string(&bytes))
}

pub fn decode_solution(solution: &str) -> Result<(Token, Vec<u64>), Error> {
    let (token, nonces) = solution.rsplit_once('.').ok_or(Error::Malformed)?;
    let token = Token::parse(token)?;
    let bytes = Base64UrlUnpadded::decode_vec(nonces).map_err(|_| Error::Malformed)?;
    if bytes.len() % 8 != 0 {
        return Err(Error::Malformed);
    }
    let nonces = bytes.as_chunks::<8>().0.iter().map(|c| u64::from_be_bytes(*c)).collect();
    Ok((token, nonces))
}

/// Single-threaded reference solver: sub-puzzles in order, nonces from zero.
pub fn solve(token: &Token) -> Vec<u64> {
    let mut hasher = Hasher::new(token);
    (0..token.challenge.k).map(|sub| hasher.search(sub, 0, u64::MAX).expect("nonce space exhausted")).collect()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Verified {
    /// Hex of the challenge seed: claim it once (e.g. Redis `SET NX`) to reject replays.
    pub replay_key: String,
    pub expires_at: u64,
}

/// Checks everything except replay, in the order: signature, expiry, scope, nonces.
pub fn verify(keys: &[&[u8]], solution: &str, scope: &Scope, now: u64) -> Result<Verified, Error> {
    let (token, nonces) = decode_solution(solution)?;
    if !keys.iter().any(|key| token.verify_mac(key)) {
        return Err(Error::BadSignature);
    }
    let challenge = &token.challenge;
    if now >= challenge.expires_at {
        return Err(Error::Expired);
    }
    if &challenge.scope != scope {
        return Err(Error::ScopeMismatch);
    }
    if nonces.len() != usize::from(challenge.k) {
        return Err(Error::WrongNonceCount);
    }
    let mut hasher = Hasher::new(&token);
    for (sub, &nonce) in nonces.iter().enumerate() {
        if !hasher.check(sub as u8, nonce) {
            return Err(Error::InvalidNonce);
        }
    }
    let replay_key = token.seed().iter().map(|b| format!("{b:02x}")).collect();
    Ok(Verified { replay_key, expires_at: challenge.expires_at })
}
