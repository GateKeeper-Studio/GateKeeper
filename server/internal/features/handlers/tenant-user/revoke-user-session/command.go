package revokeusersession

import "github.com/google/uuid"

type Command struct {
	ApplicationID uuid.UUID `json:"applicationId" validate:"required,uuid"`
	UserID        uuid.UUID `json:"userId" validate:"required,uuid"`
	SessionID     uuid.UUID `json:"sessionId" validate:"required,uuid"`
}
