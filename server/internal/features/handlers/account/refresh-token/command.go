package accountrefreshtoken

import "github.com/google/uuid"

type Command struct {
	UserID uuid.UUID `json:"-"`
}
