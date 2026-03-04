package entities

import (
	"time"

	"github.com/google/uuid"
)

// UserSession represents an active user session with device/location metadata.
type UserSession struct {
	ID            uuid.UUID
	UserID        uuid.UUID
	ApplicationID uuid.UUID
	IPAddress     string
	UserAgent     string
	Location      *string // approximate location from IP (city/country)
	CreatedAt     time.Time
	LastActiveAt  time.Time
	ExpiresAt     time.Time
	IsRevoked     bool
}

func NewUserSession(userID, applicationID uuid.UUID, ipAddress, userAgent string, location *string, ttlMinutes int) *UserSession {
	id, err := uuid.NewV7()
	if err != nil {
		panic("failed to generate UUID for UserSession")
	}

	now := time.Now().UTC()

	return &UserSession{
		ID:            id,
		UserID:        userID,
		ApplicationID: applicationID,
		IPAddress:     ipAddress,
		UserAgent:     userAgent,
		Location:      location,
		CreatedAt:     now,
		LastActiveAt:  now,
		ExpiresAt:     now.Add(time.Duration(ttlMinutes) * time.Minute),
		IsRevoked:     false,
	}
}

func (s *UserSession) IsActive() bool {
	return !s.IsRevoked && s.ExpiresAt.After(time.Now().UTC())
}
