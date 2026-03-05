package revokeusersession

import "github.com/google/uuid"

type Command struct {
	UserID    uuid.UUID `json:"userId" validate:"required,uuid"`
	SessionID uuid.UUID `json:"sessionId" validate:"required,uuid"`
}
