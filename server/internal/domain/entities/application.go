package entities

import (
	"time"

	"github.com/google/uuid"
)

// Application represents an application within the system. Each application is tied to an organization
type Application struct {
	ID                 uuid.UUID
	OrganizationID     uuid.UUID
	Name               string
	Description        *string
	CanSelfSignUp      bool
	CanSelfForgotPass  bool
	IsActive           bool
	HasMfaAuthApp      bool
	HasMfaEmail        bool
	PasswordHashSecret string
	Badges             []string
	CreatedAt          time.Time
	UpdatedAt          *time.Time
}

func NewApplication(ID uuid.UUID, name string, description *string, organizationID uuid.UUID, passwordHashSecret string, badges []string, hasMfaEmail, hasMfaAuthApp, isActive bool, updatedAt *time.Time, createdAt time.Time, canSelfSignUp, canSelfForgotPass bool) *Application {
	return &Application{
		ID:                 ID,
		OrganizationID:     organizationID,
		Name:               name,
		Description:        description,
		CreatedAt:          createdAt,
		UpdatedAt:          updatedAt,
		PasswordHashSecret: passwordHashSecret,
		IsActive:           isActive,
		HasMfaAuthApp:      hasMfaAuthApp,
		HasMfaEmail:        hasMfaEmail,
		Badges:             badges,
		CanSelfSignUp:      canSelfSignUp,
		CanSelfForgotPass:  canSelfForgotPass,
	}
}

func AddApplication(name string, description *string, organizationID uuid.UUID, passwordHashSecret string, badges []string, hasMfaEmail, hasMfaAuthApp, isActive bool, updatedAt *time.Time, canSelfSignUp, canSelfForgotPass bool) *Application {
	newID := uuid.New()

	return &Application{
		ID:                 newID,
		OrganizationID:     organizationID,
		Name:               name,
		Description:        description,
		CreatedAt:          time.Now(),
		UpdatedAt:          updatedAt,
		PasswordHashSecret: passwordHashSecret,
		IsActive:           isActive,
		HasMfaAuthApp:      hasMfaAuthApp,
		HasMfaEmail:        hasMfaEmail,
		Badges:             badges,
		CanSelfSignUp:      canSelfSignUp,
		CanSelfForgotPass:  canSelfForgotPass,
	}
}
