package getapplicationbyid

import (
	"time"

	"github.com/google/uuid"
)

type Response struct {
	ID                    uuid.UUID            `json:"id"`
	Name                  string               `json:"name"`
	Description           *string              `json:"description"`
	Badges                []string             `json:"badges"`
	CreatedAt             time.Time            `json:"createdAt"`
	UpdatedAt             *time.Time           `json:"updatedAt"`
	IsActive              bool                 `json:"isActive"`
	MfaAuthAppEnabled     bool                 `json:"mfaAuthAppEnabled"`
	MfaEmailEnabled       bool                 `json:"mfaEmailEnabled"`
	MfaWebauthnEnabled    bool                 `json:"mfaWebauthnEnabled"`
	CanSelfSignUp         bool                 `json:"canSelfSignUp"`
	CanSelfForgotPass     bool                 `json:"canSelfForgotPass"`
	PasswordHashingSecret string               `json:"passwordHashingSecret"`
	RefreshTokenTtlDays   int                  `json:"refreshTokenTtlDays"`
	Secrets               []ApplicationSecrets `json:"secrets"`
}

type ApplicationSecrets struct {
	ID             uuid.UUID  `json:"id"`
	Name           string     `json:"name"`
	Value          string     `json:"value"`
	ExpirationDate *time.Time `json:"expirationDate"`
}
