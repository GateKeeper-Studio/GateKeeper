package listtenants

import "github.com/google/uuid"

type Query struct {
	UserID uuid.UUID `json:"user_id" validate:"required"`
}
