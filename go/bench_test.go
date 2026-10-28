package pow

import (
	"fmt"
	"testing"
	"time"
)

func benchSolution(b *testing.B, p Params) (string, Scope, time.Time) {
	now := time.Unix(1_800_000_000, 0)
	scope := Scope{Site: "www.kungal.com", Action: "agent.search"}
	tok, err := Issue([]byte("bench"), p, scope, now)
	if err != nil {
		b.Fatal(err)
	}
	return tok + "." + b64.EncodeToString(make([]byte, 8*int(p.K))), scope, now
}

func BenchmarkVerify(b *testing.B) {
	configs := []Params{{Alg: SHA256, K: 16}}
	for _, m := range []uint32{256, 512, 1024, 2048, 4096} {
		for _, k := range []uint8{1, 2, 4} {
			configs = append(configs, Params{Alg: Argon2id, MemoryKiB: m, Iterations: 1, K: k})
		}
	}
	for _, p := range configs {
		p.Difficulty, p.TTL = 1, time.Hour
		name := fmt.Sprintf("sha256/k=%d", p.K)
		if p.Alg == Argon2id {
			name = fmt.Sprintf("argon2id/m=%d/k=%d", p.MemoryKiB, p.K)
		}
		b.Run(name, func(b *testing.B) {
			solution, scope, now := benchSolution(b, p)
			keys := [][]byte{[]byte("bench")}
			for b.Loop() {
				if _, err := Verify(keys, solution, scope, now); err != nil {
					b.Fatal(err)
				}
			}
		})
	}
}
