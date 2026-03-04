package me

import "github.com/google/uuid"

type Command struct {
	UserID uuid.UUID `json:"-"` // injected from JWT context
}
