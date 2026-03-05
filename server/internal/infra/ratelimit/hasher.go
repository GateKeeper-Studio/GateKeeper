package ratelimit

import (
	"crypto/sha256"
	"fmt"
)

// Hash returns a hex-encoded SHA-256 digest of s.
// Used to produce fixed-length, safe Redis keys from arbitrary input such as
// IP addresses or route patterns.
func Hash(s string) string {
	sum := sha256.Sum256([]byte(s))
	return fmt.Sprintf("%x", sum)
}
