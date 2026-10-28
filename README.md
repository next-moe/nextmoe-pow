# nextmoe-pow

Proof-of-work human verification for the NextMoe platform.

The server issues a stateless, HMAC-signed challenge. The browser solves it in Web Workers with a small WASM solver, and the server verifies the answer with a few hash evaluations and one replay check. It needs no third-party service and asks the user to do nothing.

PoW raises the cost of automated requests; it does not prove a human is present. Use it together with IP rate limits.

- Wire format and verification order: [docs/protocol.md](docs/protocol.md)
- Algorithm benchmark (M0): [docs/m0-benchmark.md](docs/m0-benchmark.md)
- Widget: [docs/widget.md](docs/widget.md)
- Flutter: [docs/flutter.md](docs/flutter.md)

## Packages

| path | what |
|---|---|
| `crates/core` | Rust reference implementation: format, solver, verifier |
| `crates/wasm` → `packages/solver` | `@nextmoe/pow-solver`: WASM solver and worker pool |
| `packages/widget` | `@nextmoe/pow-widget`: the `<nextmoe-pow>` Web Component |
| `flutter/` | `nextmoe_pow`: Flutter plugin with the native Rust solver and a `NextmoePow` widget |
| `go/` | `github.com/next-moe/nextmoe-pow/go`: issuer and verifier for Go servers |
| `testdata/vectors.json` | shared test vectors; every implementation must pass them |

## Go

```go
import pow "github.com/next-moe/nextmoe-pow/go"

token, err := pow.Issue(key, pow.Params{
	Alg: pow.Argon2id, MemoryKiB: 1024, Iterations: 1, K: 4, Difficulty: 900, TTL: 5 * time.Minute,
}, pow.Scope{Site: "www.kungal.com", Action: "agent.search"}, time.Now())

v, err := pow.VerifyOnce(ctx, store, [][]byte{key}, solution, scope, time.Now())
```

`store` implements `Claim(ctx, key, ttl) (bool, error)`; with go-redis that is `rdb.SetNX(ctx, "pow:"+key, 1, ttl).Result()`. The parameters above are placeholders until M0 sets the defaults.

## Browser

With the widget:

```html
<nextmoe-pow name="pow" challenge-url="/api/pow/challenge"></nextmoe-pow>
```

With the solver alone:

```ts
import { createPool } from '@nextmoe/pow-solver'

const pool = createPool() // start early: compiles the WASM and warms the workers
const { solution } = await pool.solve(token, { onProgress: (p) => render(p.solved / p.k) })
```

## License

AGPL-3.0-only
