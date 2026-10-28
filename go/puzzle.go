package pow

import (
	"crypto/sha256"
	"encoding/binary"
	"math"

	"golang.org/x/crypto/argon2"
)

func threshold(difficulty uint32) uint64 {
	return math.MaxUint64 / uint64(max(difficulty, 1))
}

func hashSub(c Challenge, seed [32]byte, sub uint8, nonce uint64) [32]byte {
	if c.Alg == SHA256 {
		var msg [41]byte
		copy(msg[:], seed[:])
		msg[32] = sub
		binary.BigEndian.PutUint64(msg[33:], nonce)
		return sha256.Sum256(msg[:])
	}
	var pwd [8]byte
	binary.BigEndian.PutUint64(pwd[:], nonce)
	salt := append(seed[:], sub)
	return [32]byte(argon2.IDKey(pwd[:], salt, uint32(c.Iterations), c.MemoryKiB, 1, 32))
}

func meets(hash [32]byte, difficulty uint32) bool {
	return binary.BigEndian.Uint64(hash[:8]) <= threshold(difficulty)
}
