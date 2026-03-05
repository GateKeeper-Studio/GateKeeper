package createtenantuser

import "github.com/google/uuid"

type Command struct {
	ApplicationID         uuid.UUID
	DisplayName           string
	FirstName             string
	LastName              string
	Email                 string
	IsEmailConfirmed      bool
	TemporaryPasswordHash *string
	// IsMfaAuthAppEnabled   bool
	// IsMfaEmailEnabled     bool
	Roles []uuid.UUID
}

type RequestBody struct {
	DisplayName           string  `json:"displayName" validate:"required,min=1,max=100"`
	FirstName             string  `json:"firstName" validate:"required,min=1,max=100"`
	LastName              string  `json:"lastName" validate:"required,min=1,max=100"`
	Email                 string  `json:"email" validate:"required,email"`
	IsEmailConfirmed      bool    `json:"isEmailConfirmed" validate:"required"`
	TemporaryPasswordHash *string `json:"temporaryPasswordHash" validate:"min=8,max=100"`
	// IsMfaAuthAppEnabled   bool        `json:"isMfaAuthAppEnabled" validate:"boolean"`
	// IsMfaEmailEnabled     bool        `json:"isMfaEmailEnabled" validate:"boolean"`
	Roles []uuid.UUID `json:"roles" validate:"required"`
}
