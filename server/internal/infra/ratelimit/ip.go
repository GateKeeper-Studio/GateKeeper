package ratelimit

import (
	"net"
	"net/http"
	"strings"
)

// RealIP extracts the client IP address from the request, honouring common
// reverse-proxy headers. Falls back to RemoteAddr when no header is present.
func RealIP(r *http.Request) string {
	if ip := r.Header.Get("X-Real-Ip"); ip != "" {
		return strings.TrimSpace(ip)
	}

	if forwarded := r.Header.Get("X-Forwarded-For"); forwarded != "" {
		// X-Forwarded-For may contain a comma-separated list; take the first.
		if i := strings.Index(forwarded, ","); i != -1 {
			return strings.TrimSpace(forwarded[:i])
		}
		return strings.TrimSpace(forwarded)
	}

	host, _, err := net.SplitHostPort(r.RemoteAddr)
	if err != nil {
		return r.RemoteAddr
	}
	return host
}
