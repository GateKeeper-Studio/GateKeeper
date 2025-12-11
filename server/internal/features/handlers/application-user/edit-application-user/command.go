package editapplicationuser

import (
	"github.com/google/uuid"
)

type Command struct {
	UserID                uuid.UUID
	ApplicationID         uuid.UUID
	DisplayName           string
	FirstName             string
	LastName              string
	Email                 string
	IsEmailConfirmed      bool
	TemporaryPasswordHash *string
	Roles                 []uuid.UUID
	IsActive              bool
	Preferred2FAMethod    *string
}

type RequestBody struct {
	DisplayName           string      `json:"displayName" validate:"required,min=3,max=100"`
	FirstName             string      `json:"firstName" validate:"required,min=3,max=100"`
	LastName              string      `json:"lastName" validate:"required,min=3,max=100"`
	Email                 string      `json:"email" validate:"required,email"`
	IsEmailConfirmed      bool        `json:"isEmailConfirmed" validate:"boolean"`
	TemporaryPasswordHash *string     `json:"temporaryPasswordHash"`
	Roles                 []uuid.UUID `json:"roles" validate:"required"`
	IsActive              bool        `json:"isActive" validate:"boolean"`
	Preferred2FAMethod    *string     `json:"preferred2FAMethod" validate:"omitempty,oneof=totp email sms"`
}
