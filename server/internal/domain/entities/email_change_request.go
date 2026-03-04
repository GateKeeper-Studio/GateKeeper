package entities

import (
	"time"

	"github.com/google/uuid"
)

// EmailChangeRequest represents a pending email address change that requires verification.
type EmailChangeRequest struct {
	ID            uuid.UUID
	UserID        uuid.UUID
	ApplicationID uuid.UUID
	NewEmail      string
	Token         string // signed, expiring verification token
	CreatedAt     time.Time
	ExpiresAt     time.Time
	IsConfirmed   bool
}

func NewEmailChangeRequest(userID, applicationID uuid.UUID, newEmail string) *EmailChangeRequest {
	id, err := uuid.NewV7()
	if err != nil {
		panic("failed to generate UUID for EmailChangeRequest")
	}

	now := time.Now().UTC()

	return &EmailChangeRequest{
		ID:            id,
		UserID:        userID,
		ApplicationID: applicationID,
		NewEmail:      newEmail,
		Token:         GenerateRandomString(128),
		CreatedAt:     now,
		ExpiresAt:     now.Add(24 * time.Hour), // 24-hour expiry
		IsConfirmed:   false,
	}
}

func (e *EmailChangeRequest) IsValid() bool {
	return !e.IsConfirmed && e.ExpiresAt.After(time.Now().UTC())
}
