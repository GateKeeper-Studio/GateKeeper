package accountrevokesession

import "github.com/google/uuid"

type Command struct {
	UserID    uuid.UUID `json:"-"`
	SessionID uuid.UUID `json:"-"` // from URL param
	IPAddress string    `json:"-"`
	UserAgent string    `json:"-"`
}
