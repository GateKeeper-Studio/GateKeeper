package ratelimit

import (
	"context"
	"fmt"
)

// Limiter rate-limits requests using a backing Store.
type Limiter struct {
	cfg   Config
	store Store
}

// New creates a Limiter with the given Config and Store.
func New(cfg Config, store Store) *Limiter {
	return &Limiter{cfg: cfg, store: store}
}

// Allow returns true if the given key is within its rate limit.
// The key is scoped with the configured KeyPrefix before being passed to the
// Store, so callers can pass raw values such as an IP address.
func (l *Limiter) Allow(ctx context.Context, key string) (bool, error) {
	scoped := fmt.Sprintf("%s:%s", l.cfg.KeyPrefix, key)
	return l.store.Allow(ctx, scoped, l.cfg.Requests, l.cfg.Window)
}
