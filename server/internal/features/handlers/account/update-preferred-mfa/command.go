package accountupdatepreferredmfa

import "github.com/google/uuid"

type Command struct {
	UserID          uuid.UUID `json:"-"`
	PreferredMethod *string   `json:"preferredMethod" validate:"omitempty,oneof=totp email webauthn"`
	IPAddress       string    `json:"-"`
	UserAgent       string    `json:"-"`
}
