package createtenantuser

import "github.com/google/uuid"

type Command struct {
	TenantID              uuid.UUID
	ApplicationID         uuid.UUID
	DisplayName           string
	FirstName             string
	LastName              string
	Email                 string
	IsEmailConfirmed      bool
	TemporaryPasswordHash *string
	Roles                 []uuid.UUID
}

type RequestBody struct {
	ApplicationID         uuid.UUID   `json:"applicationId" validate:"required"`
	DisplayName           string      `json:"displayName" validate:"required,min=1,max=100"`
	FirstName             string      `json:"firstName" validate:"required,min=1,max=100"`
	LastName              string      `json:"lastName" validate:"required,min=1,max=100"`
	Email                 string      `json:"email" validate:"required,email"`
	IsEmailConfirmed      bool        `json:"isEmailConfirmed" validate:"required"`
	TemporaryPasswordHash *string     `json:"temporaryPasswordHash" validate:"min=8,max=100"`
	Roles                 []uuid.UUID `json:"roles" validate:"required"`
}
