package entities

import (
	"time"

	"github.com/google/uuid"
)

type Tenant struct {
	ID          uuid.UUID
	Name        string
	Description *string
	CreatedAt   time.Time
	UpdatedAt   *time.Time
}

func NewTenant(name string, description *string) *Tenant {
	newId := uuid.New()

	return &Tenant{
		ID:          newId,
		Name:        name,
		Description: description,
		CreatedAt:   time.Now().UTC(),
	}
}
