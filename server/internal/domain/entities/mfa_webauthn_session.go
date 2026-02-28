package entities

import (
	"time"

	"github.com/google/uuid"
)

type MfaWebauthnSession struct {
	ID          uuid.UUID
	UserID      uuid.UUID
	SessionData string // JSON-encoded webauthn.SessionData
	CreatedAt   time.Time
	ExpiresAt   time.Time
}

func NewMfaWebauthnSession(userID uuid.UUID, sessionDataJSON []byte) (*MfaWebauthnSession, error) {
	id, err := uuid.NewV7()
	if err != nil {
		return nil, err
	}

	now := time.Now().UTC()
	return &MfaWebauthnSession{
		ID:          id,
		UserID:      userID,
		SessionData: string(sessionDataJSON),
		CreatedAt:   now,
		ExpiresAt:   now.Add(5 * time.Minute),
	}, nil
}

func (s *MfaWebauthnSession) IsExpired() bool {
	return time.Now().UTC().After(s.ExpiresAt)
}
