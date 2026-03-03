package listusersessions

import "github.com/google/uuid"

type Query struct {
	ApplicationID uuid.UUID `json:"applicationId" validate:"required,uuid"`
	UserID        uuid.UUID `json:"userId" validate:"required,uuid"`
}
