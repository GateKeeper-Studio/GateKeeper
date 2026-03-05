package removetenant

import "github.com/google/uuid"

type Command struct {
	ID uuid.UUID `json:"id" validate:"required"`
}
