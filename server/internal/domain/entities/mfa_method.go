package entities

import (
	"time"

	"github.com/gate-keeper/internal/domain/constants"
	"github.com/google/uuid"
)

type MfaMethod struct {
	ID         uuid.UUID
	UserID     uuid.UUID
	Type       string // e.g., "email", "sms", "totp"
	Enabled    bool
	CreatedAt  time.Time
	LastUsedAt *time.Time // Nullable, can be nil if never used
}

func AddMfaMethod(mfaMethodID uuid.UUID, mfaMethodType string) *MfaMethod {
	newId, err := uuid.NewV7()

	if err != nil {
		panic(err)
	}

	if mfaMethodType != constants.MfaMethodTotp && mfaMethodType != constants.MfaMethodEmail && mfaMethodType != constants.MfaMethodSms {
		panic("Invalid MFA method type")
	}

	return &MfaMethod{
		ID:         newId,
		UserID:     mfaMethodID,
		Type:       mfaMethodType,
		Enabled:    true,
		LastUsedAt: nil,
		CreatedAt:  time.Now().UTC(),
	}
}
