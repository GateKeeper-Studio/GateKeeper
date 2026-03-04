package http_middlewares

import (
	"net/http"
	"time"

	"github.com/gate-keeper/internal/domain/errors"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	http_router "github.com/gate-keeper/internal/presentation/http"
	"github.com/jackc/pgx/v5/pgxpool"
)

// StepUpAuthHandler is a middleware that validates a step-up token
// for sensitive operations that require reauthentication.
func StepUpAuthHandler(pool *pgxpool.Pool) func(http.Handler) http.Handler {
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			ctx := r.Context()

			stepUpToken := r.Header.Get("X-Step-Up-Token")
			if stepUpToken == "" {
				WriteJSONError(w, http.StatusForbidden, errors.ErrStepUpRequired.Title, errors.ErrStepUpRequired.Message, ctx)
				return
			}

			userID := http_router.GetUserIDFromContext(ctx)

			conn, err := pool.Acquire(ctx)
			if err != nil {
				WriteJSONError(w, http.StatusInternalServerError, "Internal Server Error", "Failed to acquire DB connection", ctx)
				return
			}
			defer conn.Release()

			queries := pgstore.New(conn)

			token, err := queries.GetStepUpTokenByToken(ctx, pgstore.GetStepUpTokenByTokenParams{
				UserID: userID,
				Token:  stepUpToken,
			})
			if err != nil {
				WriteJSONError(w, http.StatusForbidden, errors.ErrStepUpTokenNotFound.Title, errors.ErrStepUpTokenNotFound.Message, ctx)
				return
			}

			if token.ExpiresAt.Time.Before(time.Now().UTC()) {
				WriteJSONError(w, http.StatusForbidden, errors.ErrStepUpTokenExpired.Title, errors.ErrStepUpTokenExpired.Message, ctx)
				return
			}

			// Step-up token is time-based (valid for its TTL window), not single-use.
			// This allows retries if the downstream handler fails and lets the user
			// perform multiple step-up operations within the same reauthentication window.
			next.ServeHTTP(w, r)
		})
	}
}
