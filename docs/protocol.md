# Protocol v1

Stateless, HMAC-signed proof-of-work challenges. The server keeps no record of the challenges it issues. The only state it keeps is one replay key for each accepted solution, held until that challenge expires.

The contract is `testdata/vectors.json`. Rust (`crates/core`), Go (`go/`) and the WASM solver (`packages/solver`) must all pass it. Any change to the bytes below needs a new version byte and regenerated vectors.

## Challenge payload

All integers are big-endian.

| offset | size | field | notes |
|---|---|---|---|
| 0 | 1 | version | `1` |
| 1 | 1 | alg | `1` = SHA-256, `2` = Argon2id |
| 2 | 1 | k | number of sub-puzzles, 1–64 |
| 3 | 1 | iterations | Argon2id `t` (≥ 1); `0` for SHA-256 |
| 4 | 4 | memory_kib | Argon2id `m` in KiB (≥ 8); `0` for SHA-256 |
| 8 | 4 | difficulty | expected attempts per sub-puzzle, ≥ 1 |
| 12 | 8 | expires_at | Unix seconds, ≤ 2^63 − 1 |
| 20 | 16 | salt | random for each challenge |
| 36 | 1 + n | site | length-prefixed UTF-8, n ≤ 255 |
| … | 1 + n | action | length-prefixed UTF-8, n ≤ 255 |

Nothing may follow `action`. Argon2id always runs with `p = 1`, version 0x13, and a 32-byte tag.

## Strings

- `mac = HMAC-SHA256(server_key, payload)`
- **token** = `b64(payload) "." b64(mac)`, where `b64` is unpadded base64url. Decoders reject non-canonical encodings (non-zero trailing bits) and any CR or LF character.
- **solution** = `token "." b64(nonce_0 ‖ … ‖ nonce_{k−1})`, with each nonce a big-endian u64.

The client receives the token and sends back the solution. The token is opaque to the client; the solver reads its fields without checking the MAC.

## Sub-puzzles

- `seed = SHA-256(payload ‖ mac)`
- Sub-puzzle `i` (encoded as one byte) with nonce `n` (8 bytes, big-endian):
  - SHA-256: `h = SHA-256(seed ‖ i ‖ n)`, a 41-byte message.
  - Argon2id: `h = Argon2id(password = n, salt = seed ‖ i, t, m, p = 1, len = 32)`.
- A nonce is valid when `u64_be(h[0..8]) ≤ floor((2^64 − 1) / difficulty)`.

Each sub-puzzle takes `difficulty` attempts on average, with a geometric distribution. Splitting the work into `k` independent sub-puzzles turns the solve time into a sum of `k` such draws, so the spread around the mean shrinks roughly as `1/√k`. Because each draw is memoryless, several workers can search the same sub-puzzle over disjoint nonce ranges without wasting work.

## Verification

The checks run in this order. Each failure has a stable error code that is shared across implementations:

1. Decode the solution and token → `malformed`
2. MAC matches one of the accepted keys (the current key plus retired ones) → `bad_signature`
3. `now < expires_at` → `expired`
4. `site` and `action` equal the expected scope → `scope_mismatch`
5. The number of nonces equals `k` → `wrong_nonce_count`
6. Every sub-puzzle hash meets the threshold → `invalid_nonce`
7. Replay: claim `hex(seed)` once, with a TTL of `expires_at − now` (Redis `SET key 1 NX EX ttl`) → `replayed`

Steps 1–6 are `Verify`, and `VerifyOnce` adds step 7 (Go). The verify cost is `k` hash evaluations and does not depend on the difficulty.

## What it is not

PoW raises the cost of automation. It does not prove that a human is present, and an attacker with GPUs or a server farm pays far less per solve than a phone does. Always pair it with IP rate limits and global caps. The issuer is open to anyone, so rate-limit issuance as well. A server can raise `difficulty` for a client that keeps coming back.
