// Package pow issues and verifies nextmoe-pow challenges. The wire format and
// hashes match crates/core; testdata/vectors.json is the shared contract.
package pow

import (
	"crypto/hmac"
	"crypto/rand"
	"crypto/sha256"
	"encoding/base64"
	"encoding/binary"
	"errors"
	"math"
	"strings"
	"time"
)

const (
	Version  = 1
	SaltLen  = 16
	MaxK     = 64
	fixedLen = 20 + SaltLen
)

type Alg uint8

const (
	SHA256   Alg = 1
	Argon2id Alg = 2
)

type Scope struct {
	Site   string
	Action string
}

type Challenge struct {
	Alg        Alg
	MemoryKiB  uint32
	Iterations uint8
	K          uint8
	Difficulty uint32
	ExpiresAt  time.Time
	Salt       [SaltLen]byte
	Scope      Scope
}

type Params struct {
	Alg        Alg
	MemoryKiB  uint32
	Iterations uint8
	K          uint8
	Difficulty uint32
	TTL        time.Duration
}

var b64 = base64.RawURLEncoding.Strict()

func Issue(key []byte, p Params, scope Scope, now time.Time) (string, error) {
	c := Challenge{
		Alg:        p.Alg,
		MemoryKiB:  p.MemoryKiB,
		Iterations: p.Iterations,
		K:          p.K,
		Difficulty: p.Difficulty,
		ExpiresAt:  now.Add(p.TTL),
		Scope:      scope,
	}
	rand.Read(c.Salt[:])
	return c.Sign(key)
}

func (c Challenge) Sign(key []byte) (string, error) {
	payload, err := c.encode()
	if err != nil {
		return "", err
	}
	return b64.EncodeToString(payload) + "." + b64.EncodeToString(mac(key, payload)), nil
}

func (c Challenge) valid() bool {
	algOK := c.Alg == SHA256 && c.MemoryKiB == 0 && c.Iterations == 0 ||
		c.Alg == Argon2id && c.MemoryKiB >= 8 && c.Iterations >= 1
	return algOK && c.K >= 1 && c.K <= MaxK && c.Difficulty >= 1 &&
		len(c.Scope.Site) <= 255 && len(c.Scope.Action) <= 255 && c.ExpiresAt.Unix() >= 0
}

func (c Challenge) encode() ([]byte, error) {
	if !c.valid() {
		return nil, ErrMalformed
	}
	out := make([]byte, fixedLen, fixedLen+2+len(c.Scope.Site)+len(c.Scope.Action))
	out[0], out[1], out[2], out[3] = Version, byte(c.Alg), c.K, c.Iterations
	binary.BigEndian.PutUint32(out[4:], c.MemoryKiB)
	binary.BigEndian.PutUint32(out[8:], c.Difficulty)
	binary.BigEndian.PutUint64(out[12:], uint64(c.ExpiresAt.Unix()))
	copy(out[20:], c.Salt[:])
	for _, field := range []string{c.Scope.Site, c.Scope.Action} {
		out = append(out, byte(len(field)))
		out = append(out, field...)
	}
	return out, nil
}

func decodeChallenge(b []byte) (Challenge, error) {
	if len(b) < fixedLen+2 || b[0] != Version {
		return Challenge{}, ErrMalformed
	}
	expires := binary.BigEndian.Uint64(b[12:])
	if expires > math.MaxInt64 {
		return Challenge{}, ErrMalformed
	}
	c := Challenge{
		Alg:        Alg(b[1]),
		K:          b[2],
		Iterations: b[3],
		MemoryKiB:  binary.BigEndian.Uint32(b[4:]),
		Difficulty: binary.BigEndian.Uint32(b[8:]),
		ExpiresAt:  time.Unix(int64(expires), 0),
	}
	copy(c.Salt[:], b[20:fixedLen])
	rest := b[fixedLen:]
	field := func() (string, bool) {
		if len(rest) == 0 || len(rest) < 1+int(rest[0]) {
			return "", false
		}
		s := string(rest[1 : 1+rest[0]])
		rest = rest[1+rest[0]:]
		return s, true
	}
	site, ok1 := field()
	action, ok2 := field()
	c.Scope = Scope{Site: site, Action: action}
	if !ok1 || !ok2 || len(rest) != 0 || !c.valid() || !utf8Valid(site, action) {
		return Challenge{}, ErrMalformed
	}
	return c, nil
}

func mac(key, payload []byte) []byte {
	m := hmac.New(sha256.New, key)
	m.Write(payload)
	return m.Sum(nil)
}

type token struct {
	challenge Challenge
	payload   []byte
	mac       []byte
}

func parseToken(s string) (token, error) {
	payloadB64, macB64, ok := strings.Cut(s, ".")
	if !ok {
		return token{}, ErrMalformed
	}
	payload, err1 := b64.DecodeString(payloadB64)
	sig, err2 := b64.DecodeString(macB64)
	if err := errors.Join(err1, err2); err != nil || len(sig) != sha256.Size {
		return token{}, ErrMalformed
	}
	c, err := decodeChallenge(payload)
	if err != nil {
		return token{}, err
	}
	return token{challenge: c, payload: payload, mac: sig}, nil
}

func (t token) verifyMAC(key []byte) bool {
	return hmac.Equal(mac(key, t.payload), t.mac)
}

func (t token) seed() [32]byte {
	h := sha256.New()
	h.Write(t.payload)
	h.Write(t.mac)
	return [32]byte(h.Sum(nil))
}
