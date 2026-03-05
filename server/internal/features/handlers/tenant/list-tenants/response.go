package listtenants

import (
	"time"

	"github.com/google/uuid"
)

type Response struct {
	ID        uuid.UUID `json:"id"`
	Name      string    `json:"name"`
	CreatedAt time.Time `json:"createdAt"`
}
