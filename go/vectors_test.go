package pow

import (
	"context"
	"encoding/hex"
	"encoding/json"
	"errors"
	"os"
	"strconv"
	"testing"
	"time"
)

type vectorFile struct {
	Keys       map[string]string `json:"keys"`
	Challenges []struct {
		Name string `json:"name"`
		Key  string `json:"key"`
		Alg  struct {
			Name       string `json:"name"`
			MemoryKiB  uint32 `json:"memory_kib"`
			Iterations uint8  `json:"iterations"`
		} `json:"alg"`
		K          uint8  `json:"k"`
		Difficulty uint32 `json:"difficulty"`
		ExpiresAt  int64  `json:"expires_at"`
		Salt       string `json:"salt"`
		Site       string `json:"site"`
		Action     string `json:"action"`
		Payload    string `json:"payload"`
		Token      string `json:"token"`
		Seed       string `json:"seed"`
	} `json:"challenges"`
	Hashes []struct {
		Token string `json:"token"`
		Sub   uint8  `json:"sub"`
		Nonce string `json:"nonce"`
		Hash  string `json:"hash"`
	} `json:"hashes"`
	Verify []struct {
		Name      string   `json:"name"`
		Keys      []string `json:"keys"`
		Solution  string   `json:"solution"`
		Site      string   `json:"site"`
		Action    string   `json:"action"`
		Now       int64    `json:"now"`
		Expect    string   `json:"expect"`
		ReplayKey string   `json:"replay_key"`
	} `json:"verify"`
}

func loadVectors(t testing.TB) vectorFile {
	raw, err := os.ReadFile("../testdata/vectors.json")
	if err != nil {
		t.Fatal(err)
	}
	var v vectorFile
	if err := json.Unmarshal(raw, &v); err != nil {
		t.Fatal(err)
	}
	return v
}

func (v vectorFile) key(t testing.TB, name string) []byte {
	k, err := hex.DecodeString(v.Keys[name])
	if err != nil {
		t.Fatal(err)
	}
	return k
}

func TestChallengeVectors(t *testing.T) {
	v := loadVectors(t)
	for _, c := range v.Challenges {
		ch := Challenge{
			Alg:        map[string]Alg{"sha256": SHA256, "argon2id": Argon2id}[c.Alg.Name],
			MemoryKiB:  c.Alg.MemoryKiB,
			Iterations: c.Alg.Iterations,
			K:          c.K,
			Difficulty: c.Difficulty,
			ExpiresAt:  time.Unix(c.ExpiresAt, 0),
			Scope:      Scope{Site: c.Site, Action: c.Action},
		}
		salt, _ := hex.DecodeString(c.Salt)
		copy(ch.Salt[:], salt)
		payload, err := ch.encode()
		if err != nil || hex.EncodeToString(payload) != c.Payload {
			t.Errorf("%s: payload %x, %v", c.Name, payload, err)
		}
		tok, err := ch.Sign(v.key(t, c.Key))
		if err != nil || tok != c.Token {
			t.Errorf("%s: token %s, %v", c.Name, tok, err)
		}
		parsed, err := parseToken(c.Token)
		if err != nil || !parsed.challenge.ExpiresAt.Equal(ch.ExpiresAt) {
			t.Fatalf("%s: parse %v", c.Name, err)
		}
		parsed.challenge.ExpiresAt = ch.ExpiresAt
		if parsed.challenge != ch {
			t.Errorf("%s: decoded %+v", c.Name, parsed.challenge)
		}
		if seed := parsed.seed(); hex.EncodeToString(seed[:]) != c.Seed {
			t.Errorf("%s: seed %x", c.Name, seed)
		}
	}
}

func TestHashVectors(t *testing.T) {
	for _, h := range loadVectors(t).Hashes {
		tok, err := parseToken(h.Token)
		if err != nil {
			t.Fatal(err)
		}
		nonce, _ := strconv.ParseUint(h.Nonce, 10, 64)
		if got := hashSub(tok.challenge, tok.seed(), h.Sub, nonce); hex.EncodeToString(got[:]) != h.Hash {
			t.Errorf("sub %d nonce %d: %x", h.Sub, nonce, got)
		}
	}
}

func TestVerifyVectors(t *testing.T) {
	v := loadVectors(t)
	for _, c := range v.Verify {
		var keys [][]byte
		for _, k := range c.Keys {
			keys = append(keys, v.key(t, k))
		}
		got, err := Verify(keys, c.Solution, Scope{Site: c.Site, Action: c.Action}, time.Unix(c.Now, 0))
		code := "ok"
		if err != nil {
			code = err.Error()
		}
		if code != c.Expect || got.ReplayKey != c.ReplayKey {
			t.Errorf("%s: got %q %q, want %q %q", c.Name, code, got.ReplayKey, c.Expect, c.ReplayKey)
		}
	}
}

type mapStore map[string]bool

func (m mapStore) Claim(_ context.Context, key string, _ time.Duration) (bool, error) {
	first := !m[key]
	m[key] = true
	return first, nil
}

func TestVerifyOnceRejectsReplay(t *testing.T) {
	v := loadVectors(t)
	c := v.Verify[0]
	keys := [][]byte{v.key(t, "a")}
	scope := Scope{Site: c.Site, Action: c.Action}
	store := mapStore{}
	if _, err := VerifyOnce(context.Background(), store, keys, c.Solution, scope, time.Unix(c.Now, 0)); err != nil {
		t.Fatal(err)
	}
	if _, err := VerifyOnce(context.Background(), store, keys, c.Solution, scope, time.Unix(c.Now, 0)); !errors.Is(err, ErrReplayed) {
		t.Fatalf("second use: %v", err)
	}
}

func TestIssueRoundTrip(t *testing.T) {
	now := time.Unix(1_800_000_000, 0)
	scope := Scope{Site: "www.kungal.com", Action: "agent.search"}
	tok, err := Issue([]byte("k"), Params{Alg: SHA256, K: 2, Difficulty: 16, TTL: time.Minute}, scope, now)
	if err != nil {
		t.Fatal(err)
	}
	parsed, err := parseToken(tok)
	if err != nil || !parsed.verifyMAC([]byte("k")) || !parsed.challenge.ExpiresAt.Equal(now.Add(time.Minute)) {
		t.Fatalf("issued token %s: %v", tok, err)
	}
}
