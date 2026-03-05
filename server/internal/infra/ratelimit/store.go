package ratelimit

import (
	"context"
	"time"
)

// Store is the backend storage interface for the rate limiter.
type Store interface {
	// Allow returns true when the request is within the allowed limit.
	// key uniquely identifies the subject (e.g. hashed IP), limit is the
	// maximum number of requests, and window is the sliding time window.
	Allow(ctx context.Context, key string, limit int, window time.Duration) (bool, error)
}
