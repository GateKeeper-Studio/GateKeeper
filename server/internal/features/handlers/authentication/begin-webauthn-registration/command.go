package beginwebauthnregistration

import "github.com/google/uuid"

type Command struct {
	UserID        uuid.UUID `json:"userId" validate:"required"`
	ApplicationID uuid.UUID `json:"applicationId" validate:"required"`
}
