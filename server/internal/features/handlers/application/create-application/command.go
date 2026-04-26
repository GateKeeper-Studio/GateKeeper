package createapplication

import "github.com/google/uuid"

type Command struct {
	Name                 string    `json:"name" validate:"required,min=3,max=100"`
	Description          *string   `json:"description" validate:"omitempty,min=3,max=240"`
	Badges               []string  `json:"badges" validate:"required"`
	HasMfaEmail          bool      `json:"hasMfaEmail" validate:"boolean"`
	HasMfaAuthApp        bool      `json:"hasMfaAuthApp" validate:"boolean"`
	HasMfaWebauthn       bool      `json:"hasMfaWebauthn" validate:"boolean"`
	TenantID             uuid.UUID `json:"tenantId" validate:"required"`
	CanSelfSignUp        bool      `json:"canSelfSignUp" validate:"boolean"`
	CanSelfForgotPass    bool      `json:"canSelfForgotPass" validate:"boolean"`
	RequiresHighSecurity bool      `json:"requiresHighSecurity" validate:"boolean"`
}
