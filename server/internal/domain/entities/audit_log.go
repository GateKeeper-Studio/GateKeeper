package entities

import (
	"time"

	"github.com/google/uuid"
)

// AuditLog represents an immutable security audit log entry.
type AuditLog struct {
	ID            uuid.UUID
	UserID        uuid.UUID
	ApplicationID uuid.UUID
	EventType     string // e.g., "PASSWORD_CHANGED", "MFA_ENABLED"
	IPAddress     string
	UserAgent     string
	Result        string // "success" or "failure"
	Details       *string
	CreatedAt     time.Time
}

func NewAuditLog(userID, applicationID uuid.UUID, eventType, ipAddress, userAgent, result string, details *string) *AuditLog {
	id, err := uuid.NewV7()
	if err != nil {
		panic("failed to generate UUID for AuditLog")
	}

	return &AuditLog{
		ID:            id,
		UserID:        userID,
		ApplicationID: applicationID,
		EventType:     eventType,
		IPAddress:     ipAddress,
		UserAgent:     userAgent,
		Result:        result,
		Details:       details,
		CreatedAt:     time.Now().UTC(),
	}
}
