use nextmoe_pow::{Alg, Hasher, Token, encode_solution};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct Solver {
    token: Token,
    hasher: Hasher,
}

#[wasm_bindgen]
impl Solver {
    #[wasm_bindgen(constructor)]
    pub fn new(token: &str) -> Result<Solver, JsError> {
        let token = Token::parse(token).map_err(|e| JsError::new(e.code()))?;
        let hasher = Hasher::new(&token);
        Ok(Solver { token, hasher })
    }

    #[wasm_bindgen(getter)]
    pub fn alg(&self) -> String {
        match self.token.challenge.alg {
            Alg::Sha256 => "sha256".into(),
            Alg::Argon2id { .. } => "argon2id".into(),
        }
    }

    #[wasm_bindgen(getter, js_name = memoryKib)]
    pub fn memory_kib(&self) -> u32 {
        match self.token.challenge.alg {
            Alg::Sha256 => 0,
            Alg::Argon2id { memory_kib, .. } => memory_kib,
        }
    }

    #[wasm_bindgen(getter)]
    pub fn iterations(&self) -> u8 {
        match self.token.challenge.alg {
            Alg::Sha256 => 0,
            Alg::Argon2id { iterations, .. } => iterations,
        }
    }

    #[wasm_bindgen(getter)]
    pub fn k(&self) -> u8 {
        self.token.challenge.k
    }

    #[wasm_bindgen(getter)]
    pub fn difficulty(&self) -> u32 {
        self.token.challenge.difficulty
    }

    #[wasm_bindgen(getter, js_name = expiresAt)]
    pub fn expires_at(&self) -> f64 {
        self.token.challenge.expires_at as f64
    }

    #[wasm_bindgen(getter)]
    pub fn site(&self) -> String {
        self.token.challenge.scope.site.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn action(&self) -> String {
        self.token.challenge.scope.action.clone()
    }

    /// Nonces travel as JS numbers; callers keep them below 2^53.
    pub fn search(&mut self, sub: u8, start: f64, count: u32) -> Option<f64> {
        self.hasher.search(sub, start as u64, u64::from(count)).map(|n| n as f64)
    }

    pub fn hash(&mut self, sub: u8, nonce: u64) -> Vec<u8> {
        self.hasher.hash(sub, nonce).to_vec()
    }

    pub fn solution(&self, nonces: Vec<f64>) -> String {
        let nonces: Vec<u64> = nonces.into_iter().map(|n| n as u64).collect();
        encode_solution(&self.token, &nonces)
    }
}
