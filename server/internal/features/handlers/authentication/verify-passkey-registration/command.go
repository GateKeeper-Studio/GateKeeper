package verifypasskeyregistration

import (
	"encoding/json"

	"github.com/google/uuid"
)

type Command struct {
	UserID         uuid.UUID       `json:"userId" validate:"required"`
	ApplicationID  uuid.UUID       `json:"applicationId" validate:"required"`
	SessionID      uuid.UUID       `json:"sessionId" validate:"required"`
	CredentialData json.RawMessage `json:"credentialData" validate:"required"`
}
