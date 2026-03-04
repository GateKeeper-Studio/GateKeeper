package updateapplication

import "github.com/google/uuid"

type Command struct {
	ID                   uuid.UUID `json:"id" validate:"required"`
	Name                 string    `json:"name" validate:"required,min=3,max=100"`
	Description          *string   `json:"description" validate:"omitempty,min=3,max=100"`
	Badges               []string  `json:"badges" validate:"required"`
	HasMfaEmail          bool      `json:"hasMfaEmail" validate:"boolean"`
	HasMfaAuthApp        bool      `json:"hasMfaAuthApp" validate:"boolean"`
	OrganizationID       uuid.UUID `json:"organizationId" validate:"required"`
	IsActive             bool      `json:"isActive" validate:"required"`
	CanSelfSignUp        bool      `json:"canSelfSignUp" validate:"boolean"`
	CanSelfForgotPass    bool      `json:"canSelfForgotPass" validate:"boolean"`
	RefreshTokenTTLDays  int       `json:"refreshTokenTtlDays" validate:"min=1,max=365"`
	RequiresHighSecurity bool      `json:"requiresHighSecurity" validate:"boolean"`
}
