package entities

import (
	"time"

	"github.com/google/uuid"
)

type Tenant struct {
	ID                 uuid.UUID
	Name               string
	Description        *string
	CanSelfSignUp      bool
	CanSelfForgotPass  bool
	IsActive           bool
	PasswordHashSecret string
	CreatedAt          time.Time
	UpdatedAt          *time.Time
}

func NewTenant(name string, description *string, passwordHashSecret string) *Tenant {
	newId := uuid.New()

	return &Tenant{
		ID:                 newId,
		Name:               name,
		Description:        description,
		PasswordHashSecret: passwordHashSecret,
		CreatedAt:          time.Now().UTC(),
	}
}
