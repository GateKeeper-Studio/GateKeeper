package entities

import (
	"time"

	"github.com/google/uuid"
)

// StepUpToken represents a time-limited reauthentication token
// that proves the user recently verified their identity.
type StepUpToken struct {
	ID            uuid.UUID
	UserID        uuid.UUID
	ApplicationID uuid.UUID
	Token         string
	CreatedAt     time.Time
	ExpiresAt     time.Time
	IsUsed        bool
}

func NewStepUpToken(userID, applicationID uuid.UUID) *StepUpToken {
	id, err := uuid.NewV7()
	if err != nil {
		panic("failed to generate UUID for StepUpToken")
	}

	now := time.Now().UTC()

	return &StepUpToken{
		ID:            id,
		UserID:        userID,
		ApplicationID: applicationID,
		Token:         GenerateRandomString(64),
		CreatedAt:     now,
		ExpiresAt:     now.Add(5 * time.Minute), // 5-minute validity
		IsUsed:        false,
	}
}

func (s *StepUpToken) IsValid() bool {
	return !s.IsUsed && s.ExpiresAt.After(time.Now().UTC())
}
