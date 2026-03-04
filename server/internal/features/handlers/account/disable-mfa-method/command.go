package accountdisablemfamethod

import "github.com/google/uuid"

type Command struct {
	UserID    uuid.UUID `json:"-"`
	Method    string    `json:"-" validate:"required,oneof=totp email webauthn"`
	IPAddress string    `json:"-"`
	UserAgent string    `json:"-"`
}
