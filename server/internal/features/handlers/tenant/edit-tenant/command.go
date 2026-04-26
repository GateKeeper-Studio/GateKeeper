package edittenant

import "github.com/google/uuid"

type Command struct {
	ID                 uuid.UUID
	Name               string
	Description        *string
	PasswordHashSecret *string
}

type RequestBody struct {
	Name               string  `json:"name" validate:"required"`
	Description        *string `json:"description" validate:"omitempty"`
	PasswordHashSecret *string `json:"passwordHashSecret" validate:"omitempty,min=32,max=258"`
}
