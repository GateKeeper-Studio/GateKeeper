package deletetenantuser

import (
	"github.com/google/uuid"
)

type Command struct {
	TenantID uuid.UUID
	UserID   uuid.UUID
}
