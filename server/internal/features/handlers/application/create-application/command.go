package createapplication

import "github.com/google/uuid"

type Command struct {
	Name                 string    `json:"name" validate:"required,min=3,max=100"`
	Description          *string   `json:"description" validate:"omitempty,min=3,max=240"`
	PasswordHashSecret   string    `json:"passwordHashSecret" validate:"required,min=32,max=258"`
	Badges               []string  `json:"badges" validate:"required"`
	HasMfaEmail          bool      `json:"hasMfaEmail" validate:"boolean"`
	HasMfaAuthApp        bool      `json:"hasMfaAuthApp" validate:"boolean"`
	HasMfaWebauthn       bool      `json:"hasMfaWebauthn" validate:"boolean"`
	OrganizationID       uuid.UUID `json:"organizationId" validate:"required"`
	CanSelfSignUp        bool      `json:"canSelfSignUp" validate:"boolean"`
	CanSelfForgotPass    bool      `json:"canSelfForgotPass" validate:"boolean"`
	RequiresHighSecurity bool      `json:"requiresHighSecurity" validate:"boolean"`
}
