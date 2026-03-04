package http_router

import (
	"context"

	"github.com/google/uuid"
)

type contextKey string

const (
	UserIDKey        contextKey = "userId"
	ApplicationIDKey contextKey = "applicationId"
)

// GetUserIDFromContext extracts the authenticated user's ID from the request context.
// This value is set by the JWT middleware and must never be trusted from the client.
func GetUserIDFromContext(ctx context.Context) uuid.UUID {
	userIDStr, ok := ctx.Value(UserIDKey).(string)
	if !ok {
		panic("user ID not found in context")
	}
	return uuid.MustParse(userIDStr)
}

// GetApplicationIDFromContext extracts the application ID from the request context if available.
func GetApplicationIDFromContext(ctx context.Context) uuid.UUID {
	appIDStr, ok := ctx.Value(ApplicationIDKey).(string)
	if !ok {
		return uuid.Nil
	}
	return uuid.MustParse(appIDStr)
}
