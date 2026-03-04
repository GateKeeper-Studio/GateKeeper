package accountgeneratebackupcodes

import "github.com/google/uuid"

type Command struct {
	UserID        uuid.UUID `json:"-"`
	ApplicationID uuid.UUID `json:"applicationId" validate:"required"`
	IPAddress     string    `json:"-"`
	UserAgent     string    `json:"-"`
}
