use base64ct::{Base64UrlUnpadded, Encoding};
use hmac::{Hmac, KeyInit, Mac};
use sha2::{Digest, Sha256};

use crate::Error;

pub const VERSION: u8 = 1;
pub const SALT_LEN: usize = 16;
pub const MAX_K: u8 = 64;
const FIXED_LEN: usize = 20 + SALT_LEN;
const ALG_SHA256: u8 = 1;
const ALG_ARGON2ID: u8 = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Alg {
    Sha256,
    Argon2id { memory_kib: u32, iterations: u8 },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Scope {
    pub site: String,
    pub action: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Challenge {
    pub alg: Alg,
    pub k: u8,
    pub difficulty: u32,
    pub expires_at: u64,
    pub salt: [u8; SALT_LEN],
    pub scope: Scope,
}

impl Challenge {
    pub fn encode(&self) -> Result<Vec<u8>, Error> {
        self.validate()?;
        let (alg, iterations, memory_kib) = match self.alg {
            Alg::Sha256 => (ALG_SHA256, 0, 0),
            Alg::Argon2id { memory_kib, iterations } => (ALG_ARGON2ID, iterations, memory_kib),
        };
        let mut out = Vec::with_capacity(FIXED_LEN + 2 + self.scope.site.len() + self.scope.action.len());
        out.extend_from_slice(&[VERSION, alg, self.k, iterations]);
        out.extend_from_slice(&memory_kib.to_be_bytes());
        out.extend_from_slice(&self.difficulty.to_be_bytes());
        out.extend_from_slice(&self.expires_at.to_be_bytes());
        out.extend_from_slice(&self.salt);
        for field in [&self.scope.site, &self.scope.action] {
            out.push(field.len() as u8);
            out.extend_from_slice(field.as_bytes());
        }
        Ok(out)
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() < FIXED_LEN + 2 || bytes[0] != VERSION {
            return Err(Error::Malformed);
        }
        let u32_at = |i: usize| u32::from_be_bytes(bytes[i..i + 4].try_into().unwrap());
        let (iterations, memory_kib) = (bytes[3], u32_at(4));
        let alg = match bytes[1] {
            ALG_SHA256 if iterations == 0 && memory_kib == 0 => Alg::Sha256,
            ALG_ARGON2ID => Alg::Argon2id { memory_kib, iterations },
            _ => return Err(Error::Malformed),
        };
        let mut rest = &bytes[FIXED_LEN..];
        let mut field = || -> Result<String, Error> {
            let (&len, tail) = rest.split_first().ok_or(Error::Malformed)?;
            let (value, tail) = tail.split_at_checked(len as usize).ok_or(Error::Malformed)?;
            rest = tail;
            String::from_utf8(value.to_vec()).map_err(|_| Error::Malformed)
        };
        let scope = Scope { site: field()?, action: field()? };
        if !rest.is_empty() {
            return Err(Error::Malformed);
        }
        let challenge = Challenge {
            alg,
            k: bytes[2],
            difficulty: u32_at(8),
            expires_at: u64::from_be_bytes(bytes[12..20].try_into().unwrap()),
            salt: bytes[20..FIXED_LEN].try_into().unwrap(),
            scope,
        };
        challenge.validate()?;
        Ok(challenge)
    }

    pub fn sign(&self, key: &[u8]) -> Result<String, Error> {
        let payload = self.encode()?;
        let mac = mac(key, &payload);
        Ok(format!("{}.{}", Base64UrlUnpadded::encode_string(&payload), Base64UrlUnpadded::encode_string(&mac)))
    }

    fn validate(&self) -> Result<(), Error> {
        let alg_ok = match self.alg {
            Alg::Sha256 => true,
            Alg::Argon2id { memory_kib, iterations } => memory_kib >= 8 && iterations >= 1,
        };
        let fields_ok =
            self.expires_at <= i64::MAX as u64 && self.scope.site.len() <= 255 && self.scope.action.len() <= 255;
        if alg_ok && fields_ok && (1..=MAX_K).contains(&self.k) && self.difficulty >= 1 {
            Ok(())
        } else {
            Err(Error::Malformed)
        }
    }
}

fn mac(key: &[u8], payload: &[u8]) -> [u8; 32] {
    let mut m = Hmac::<Sha256>::new_from_slice(key).expect("HMAC accepts any key length");
    m.update(payload);
    m.finalize().into_bytes().into()
}

/// A parsed challenge token. Parsing does not check the signature; the solver
/// has no key, and the verifier calls [`Token::verify_mac`] first.
#[derive(Clone, Debug)]
pub struct Token {
    pub challenge: Challenge,
    payload: Vec<u8>,
    mac: [u8; 32],
}

impl Token {
    pub fn parse(token: &str) -> Result<Self, Error> {
        let (payload, mac) = token.split_once('.').ok_or(Error::Malformed)?;
        let payload = Base64UrlUnpadded::decode_vec(payload).map_err(|_| Error::Malformed)?;
        let mut mac_bytes = [0u8; 32];
        let decoded = Base64UrlUnpadded::decode(mac, &mut mac_bytes).map_err(|_| Error::Malformed)?;
        if decoded.len() != 32 {
            return Err(Error::Malformed);
        }
        Ok(Token { challenge: Challenge::decode(&payload)?, payload, mac: mac_bytes })
    }

    pub fn verify_mac(&self, key: &[u8]) -> bool {
        let mut m = Hmac::<Sha256>::new_from_slice(key).expect("HMAC accepts any key length");
        m.update(&self.payload);
        m.verify_slice(&self.mac).is_ok()
    }

    /// Per-challenge puzzle seed; its hex form is also the replay key.
    pub fn seed(&self) -> [u8; 32] {
        Sha256::new().chain_update(&self.payload).chain_update(self.mac).finalize().into()
    }

    pub fn encode(&self) -> String {
        format!("{}.{}", Base64UrlUnpadded::encode_string(&self.payload), Base64UrlUnpadded::encode_string(&self.mac))
    }
}
