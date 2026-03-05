package ratelimit

import "time"

// Config holds the rate limiter configuration.
type Config struct {
	// Requests is the maximum number of requests allowed within Window.
	Requests int
	// Window is the duration of the sliding rate-limit window.
	Window time.Duration
	// KeyPrefix is prepended to every Redis key to avoid collisions.
	KeyPrefix string
}
