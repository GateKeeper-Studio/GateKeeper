package verifypasskeyauth

import (
	"encoding/json"

	"github.com/google/uuid"
)

type Command struct {
	Email         string          `json:"email" validate:"required,email"`
	ApplicationID uuid.UUID       `json:"applicationId" validate:"required"`
	SessionID     uuid.UUID       `json:"sessionId" validate:"required"`
	AssertionData json.RawMessage `json:"assertionData" validate:"required"`
}
