package gettenantbyid

import "github.com/google/uuid"

type Query struct {
	TenantID uuid.UUID
}
