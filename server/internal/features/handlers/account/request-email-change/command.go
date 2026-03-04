package accountrequestemailchange

import "github.com/google/uuid"

type Command struct {
	UserID        uuid.UUID `json:"-"`
	ApplicationID uuid.UUID `json:"applicationId" validate:"required"`
	NewEmail      string    `json:"newEmail" validate:"required,email"`
	IPAddress     string    `json:"-"`
	UserAgent     string    `json:"-"`
}
