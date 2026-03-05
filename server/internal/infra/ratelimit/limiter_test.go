package ratelimit_test

import (
	"context"
	"testing"
	"time"

	"github.com/gate-keeper/internal/infra/ratelimit"
)

// mockStore is an in-memory Store used in unit tests. It counts calls per key
// and enforces the limit passed to Allow.
type mockStore struct {
	counters map[string]int
}

func newMockStore() *mockStore {
	return &mockStore{counters: make(map[string]int)}
}

func (m *mockStore) Allow(_ context.Context, key string, limit int, _ time.Duration) (bool, error) {
	m.counters[key]++
	return m.counters[key] <= limit, nil
}

func TestLimiter_AllowsUpToLimit(t *testing.T) {
	store := newMockStore()
	cfg := ratelimit.Config{
		Requests:  3,
		Window:    time.Minute,
		KeyPrefix: "test",
	}
	limiter := ratelimit.New(cfg, store)
	ctx := context.Background()

	for i := 1; i <= 3; i++ {
		ok, err := limiter.Allow(ctx, "client-1")
		if err != nil {
			t.Fatalf("request %d: unexpected error: %v", i, err)
		}
		if !ok {
			t.Fatalf("request %d: expected allowed, got denied", i)
		}
	}
}

func TestLimiter_DeniesOverLimit(t *testing.T) {
	store := newMockStore()
	cfg := ratelimit.Config{
		Requests:  2,
		Window:    time.Minute,
		KeyPrefix: "test",
	}
	limiter := ratelimit.New(cfg, store)
	ctx := context.Background()

	limiter.Allow(ctx, "client-1") //nolint:errcheck
	limiter.Allow(ctx, "client-1") //nolint:errcheck

	ok, err := limiter.Allow(ctx, "client-1")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if ok {
		t.Fatal("expected denied on third request, got allowed")
	}
}

func TestLimiter_IndependentKeys(t *testing.T) {
	store := newMockStore()
	cfg := ratelimit.Config{
		Requests:  2,
		Window:    time.Minute,
		KeyPrefix: "test",
	}
	limiter := ratelimit.New(cfg, store)
	ctx := context.Background()

	// Each client should have its own independent counter.
	for _, client := range []string{"client-a", "client-b"} {
		for i := 1; i <= 2; i++ {
			ok, err := limiter.Allow(ctx, client)
			if err != nil || !ok {
				t.Fatalf("client %s, request %d: expected allowed (err=%v, ok=%v)", client, i, err, ok)
			}
		}
	}
}

func TestLimiter_KeyScoping(t *testing.T) {
	store := newMockStore()
	cfg := ratelimit.Config{
		Requests:  1,
		Window:    time.Minute,
		KeyPrefix: "myapp",
	}
	limiter := ratelimit.New(cfg, store)
	ctx := context.Background()

	limiter.Allow(ctx, "ip-1") //nolint:errcheck

	// The prefix "myapp" must be part of the key, so a second limiter with a
	// different prefix for the same raw key should be independent.
	cfg2 := ratelimit.Config{Requests: 1, Window: time.Minute, KeyPrefix: "otherapp"}
	limiter2 := ratelimit.New(cfg2, store)

	ok, err := limiter2.Allow(ctx, "ip-1")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if !ok {
		t.Fatal("expected allowed for different prefix, got denied")
	}
}
