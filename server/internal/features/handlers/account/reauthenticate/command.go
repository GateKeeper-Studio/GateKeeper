package reauthenticate

import "github.com/google/uuid"

type Command struct {
	UserID        uuid.UUID `json:"-"` // injected from JWT context, never from client
	ApplicationID uuid.UUID `json:"applicationId" validate:"required"`
	Password      string    `json:"password" validate:"required"`
	TOTPCode      *string   `json:"totpCode" validate:"omitempty"`
}
