package pow

import (
	"context"
	"encoding/binary"
	"encoding/hex"
	"strings"
	"time"
	"unicode/utf8"
)

type verifyError string

func (e verifyError) Error() string { return string(e) }

const (
	ErrMalformed       verifyError = "malformed"
	ErrBadSignature    verifyError = "bad_signature"
	ErrExpired         verifyError = "expired"
	ErrScopeMismatch   verifyError = "scope_mismatch"
	ErrWrongNonceCount verifyError = "wrong_nonce_count"
	ErrInvalidNonce    verifyError = "invalid_nonce"
	ErrReplayed        verifyError = "replayed"
)

type Verified struct {
	// ReplayKey is the hex challenge seed; claim it once to reject replays.
	ReplayKey string
	ExpiresAt time.Time
}

// ReplayStore claims a key until ttl elapses and reports whether this call
// was first. With go-redis: rdb.SetNX(ctx, "pow:"+key, 1, ttl).Result().
type ReplayStore interface {
	Claim(ctx context.Context, key string, ttl time.Duration) (bool, error)
}

// Verify checks signature, expiry, scope and nonces, in that order; it does
// not check replay. keys[0] is the current key, the rest are still accepted.
func Verify(keys [][]byte, solution string, scope Scope, now time.Time) (Verified, error) {
	if strings.ContainsAny(solution, "\r\n") {
		return Verified{}, ErrMalformed
	}
	cut := strings.LastIndexByte(solution, '.')
	if cut < 0 {
		return Verified{}, ErrMalformed
	}
	t, err := parseToken(solution[:cut])
	if err != nil {
		return Verified{}, err
	}
	raw, err := b64.DecodeString(solution[cut+1:])
	if err != nil || len(raw)%8 != 0 {
		return Verified{}, ErrMalformed
	}
	signed := false
	for _, key := range keys {
		signed = signed || t.verifyMAC(key)
	}
	if !signed {
		return Verified{}, ErrBadSignature
	}
	c := t.challenge
	if !now.Before(c.ExpiresAt) {
		return Verified{}, ErrExpired
	}
	if c.Scope != scope {
		return Verified{}, ErrScopeMismatch
	}
	if len(raw)/8 != int(c.K) {
		return Verified{}, ErrWrongNonceCount
	}
	seed := t.seed()
	for sub := range c.K {
		if !meets(hashSub(c, seed, sub, binary.BigEndian.Uint64(raw[8*int(sub):])), c.Difficulty) {
			return Verified{}, ErrInvalidNonce
		}
	}
	return Verified{ReplayKey: hex.EncodeToString(seed[:]), ExpiresAt: c.ExpiresAt}, nil
}

// VerifyOnce is Verify followed by a replay claim held until the challenge expires.
func VerifyOnce(ctx context.Context, store ReplayStore, keys [][]byte, solution string, scope Scope, now time.Time) (Verified, error) {
	v, err := Verify(keys, solution, scope, now)
	if err != nil {
		return Verified{}, err
	}
	first, err := store.Claim(ctx, v.ReplayKey, v.ExpiresAt.Sub(now))
	if err != nil {
		return Verified{}, err
	}
	if !first {
		return Verified{}, ErrReplayed
	}
	return v, nil
}

func utf8Valid(s ...string) bool {
	for _, x := range s {
		if !utf8.ValidString(x) {
			return false
		}
	}
	return true
}
