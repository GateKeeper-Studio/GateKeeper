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
	HasMfaPasskey        bool
	RequiresHighSecurity bool
	Badges               []string
	RefreshTokenTTLDays  int
	CreatedAt            time.Time
	UpdatedAt            *time.Time
}

func NewApplication(ID uuid.UUID, name string, description *string, tenantID uuid.UUID, badges []string, hasMfaEmail, hasMfaAuthApp, hasMfaPasskey, isActive bool, updatedAt *time.Time, createdAt time.Time, canSelfSignUp, canSelfForgotPass bool, refreshTokenTTLDays int, requiresHighSecurity bool) *Application {
	return &Application{
		ID:                   ID,
		TenantID:             tenantID,
		Name:                 name,
		Description:          description,
		CreatedAt:            createdAt,
		UpdatedAt:            updatedAt,
		IsActive:             isActive,
		HasMfaAuthApp:        hasMfaAuthApp,
		HasMfaEmail:          hasMfaEmail,
		HasMfaPasskey:        hasMfaPasskey,
		RequiresHighSecurity: requiresHighSecurity,
		Badges:               badges,
		RefreshTokenTTLDays:  refreshTokenTTLDays,
		CanSelfSignUp:        canSelfSignUp,
		CanSelfForgotPass:    canSelfForgotPass,
	}
}

func AddApplication(name string, description *string, tenantID uuid.UUID, badges []string, hasMfaEmail, hasMfaAuthApp, hasMfaPasskey, isActive bool, updatedAt *time.Time, canSelfSignUp, canSelfForgotPass bool) *Application {
	newID := uuid.New()

	return &Application{
		ID:                newID,
		TenantID:          tenantID,
		Name:              name,
		Description:       description,
		CreatedAt:         time.Now(),
		UpdatedAt:         updatedAt,
		IsActive:          isActive,
		HasMfaAuthApp:     hasMfaAuthApp,
		HasMfaEmail:       hasMfaEmail,
		HasMfaPasskey:     hasMfaPasskey,
		Badges:            badges,
		CanSelfSignUp:     canSelfSignUp,
		CanSelfForgotPass: canSelfForgotPass,
	}
}
