package beginwebauthnregistration

import (
	"encoding/json"

	"github.com/google/uuid"
)

type Response struct {
	SessionID uuid.UUID       `json:"sessionId"`
	Options   json.RawMessage `json:"options"`
}
