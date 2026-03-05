package getapplicationbyid

import "github.com/google/uuid"

type Query struct {
	ApplicationID uuid.UUID `json:"applicationId" validate:"required"`
	TenantID      uuid.UUID `json:"tenantId" validate:"required"`
}
