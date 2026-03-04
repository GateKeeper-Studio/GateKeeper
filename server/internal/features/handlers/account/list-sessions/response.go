package accountlistsessions

import (
	"time"

	"github.com/google/uuid"
)

// SessionItem represents a single active session for display.
// Never exposes raw session IDs — uses an opaque identifier.
type SessionItem struct {
	ID           uuid.UUID `json:"id"`
	IPAddress    string    `json:"ipAddress"`
	UserAgent    string    `json:"userAgent"`
	Location     *string   `json:"location"`
	CreatedAt    time.Time `json:"createdAt"`
	LastActiveAt time.Time `json:"lastActiveAt"`
	ExpiresAt    time.Time `json:"expiresAt"`
}

type Response struct {
	Sessions []SessionItem `json:"sessions"`
	Total    int           `json:"total"`
}
