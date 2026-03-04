package accountchangepassword

import "github.com/google/uuid"

type Command struct {
	UserID          uuid.UUID `json:"-"` // injected from JWT context, never from client
	ApplicationID   uuid.UUID `json:"applicationId" validate:"required"`
	CurrentPassword string    `json:"currentPassword" validate:"required"`
	NewPassword     string    `json:"newPassword" validate:"required,min=8,max=128"`
	IPAddress       string    `json:"-"` // injected server-side
	UserAgent       string    `json:"-"` // injected server-side
}
