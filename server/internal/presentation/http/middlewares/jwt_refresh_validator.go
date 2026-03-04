package http_middlewares

import (
	"context"
	"net/http"
	"strings"
	"time"

	application_utils "github.com/gate-keeper/internal/features/utils"
	http_router "github.com/gate-keeper/internal/presentation/http"
)

// JwtRefreshHandler validates a JWT with a 30-minute leeway for token expiration.
// This allows the refresh endpoint to accept recently-expired tokens so that
// clients can obtain a fresh access token without forcing a full re-login.
func JwtRefreshHandler(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		ctx := r.Context()

		authHeader := r.Header.Get("Authorization")

		if authHeader == "" {
			WriteJSONError(w, http.StatusUnauthorized, "Unauthorized", "Missing token", ctx)
			return
		}

		jwtTokenParts := strings.Split(authHeader, "Bearer ")

		if len(jwtTokenParts) != 2 {
			WriteJSONError(w, http.StatusUnauthorized, "Unauthorized", "Invalid token", ctx)
			return
		}

		jwtToken := jwtTokenParts[1]
		isValid, userID, err := application_utils.ValidateTokenWithLeeway(jwtToken, 30*time.Minute)

		if err != nil {
			WriteJSONError(w, http.StatusUnauthorized, "Unauthorized", err.Error(), ctx)
			return
		}

		if !isValid {
			WriteJSONError(w, http.StatusUnauthorized, "Unauthorized", "Invalid token", ctx)
			return
		}

		ctx = context.WithValue(ctx, http_router.UserIDKey, userID)
		r = r.WithContext(ctx)

		next.ServeHTTP(w, r)
	})
}
