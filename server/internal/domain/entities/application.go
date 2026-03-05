package entities

import (
	"time"

	"github.com/google/uuid"
)

// Application represents an application within the system. Each application is tied to an tenant
type Application struct {
	ID                   uuid.UUID
	TenantID             uuid.UUID
	Name                 string
	Description          *string
	CanSelfSignUp        bool
	CanSelfForgotPass    bool
	IsActive             bool
	HasMfaAuthApp        bool
	HasMfaEmail          bool
	HasMfaWebauthn       bool
	RequiresHighSecurity bool
	PasswordHashSecret   string
	Badges               []string
	RefreshTokenTTLDays  int
	CreatedAt            time.Time
	UpdatedAt            *time.Time
}

func NewApplication(ID uuid.UUID, name string, description *string, tenantID uuid.UUID, passwordHashSecret string, badges []string, hasMfaEmail, hasMfaAuthApp, hasMfaWebauthn, isActive bool, updatedAt *time.Time, createdAt time.Time, canSelfSignUp, canSelfForgotPass bool, refreshTokenTTLDays int, requiresHighSecurity bool) *Application {
	return &Application{
		ID:                   ID,
		TenantID:             tenantID,
		Name:                 name,
		Description:          description,
		CreatedAt:            createdAt,
		UpdatedAt:            updatedAt,
		PasswordHashSecret:   passwordHashSecret,
		IsActive:             isActive,
		HasMfaAuthApp:        hasMfaAuthApp,
		HasMfaEmail:          hasMfaEmail,
		HasMfaWebauthn:       hasMfaWebauthn,
		RequiresHighSecurity: requiresHighSecurity,
		Badges:               badges,
		RefreshTokenTTLDays:  refreshTokenTTLDays,
		CanSelfSignUp:        canSelfSignUp,
		CanSelfForgotPass:    canSelfForgotPass,
	}
}

func AddApplication(name string, description *string, tenantID uuid.UUID, passwordHashSecret string, badges []string, hasMfaEmail, hasMfaAuthApp, hasMfaWebauthn, isActive bool, updatedAt *time.Time, canSelfSignUp, canSelfForgotPass bool) *Application {
	newID := uuid.New()

	return &Application{
		ID:                 newID,
		TenantID:           tenantID,
		Name:               name,
		Description:        description,
		CreatedAt:          time.Now(),
		UpdatedAt:          updatedAt,
		PasswordHashSecret: passwordHashSecret,
		IsActive:           isActive,
		HasMfaAuthApp:      hasMfaAuthApp,
		HasMfaEmail:        hasMfaEmail,
		HasMfaWebauthn:     hasMfaWebauthn,
		Badges:             badges,
		CanSelfSignUp:      canSelfSignUp,
		CanSelfForgotPass:  canSelfForgotPass,
	}
}
