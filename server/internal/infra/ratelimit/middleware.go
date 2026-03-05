package ratelimit

import (
	"encoding/json"
	"net/http"
)

// Middleware returns a chi-compatible HTTP middleware that rate-limits requests
// by client IP address. When a client exceeds the configured limit the handler
// responds with HTTP 429 Too Many Requests and a JSON error body.
//
// On store errors the request is allowed through (fail-open) to avoid blocking
// legitimate traffic due to transient Redis issues.
func Middleware(limiter *Limiter) func(http.Handler) http.Handler {
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			ip := RealIP(r)
			key := Hash(ip)

			allowed, err := limiter.Allow(r.Context(), key)
			if err != nil {
				// Fail open: let the request proceed rather than denying
				// legitimate traffic on a storage error.
				next.ServeHTTP(w, r)
				return
			}

			if !allowed {
				w.Header().Set("Content-Type", "application/json")
				w.WriteHeader(http.StatusTooManyRequests)
				json.NewEncoder(w).Encode(map[string]string{ //nolint:errcheck
					"error": "too many requests",
				})
				return
			}

			next.ServeHTTP(w, r)
		})
	}
}
