package listapplications

import "github.com/google/uuid"

type Query struct {
	TenantID uuid.UUID `json:"tenantId" validate:"required"`
}
