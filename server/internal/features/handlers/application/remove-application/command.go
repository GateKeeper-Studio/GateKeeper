package removeapplication

import "github.com/google/uuid"

type Command struct {
	ApplicationID uuid.UUID `json:"applicationId" validate:"required"`
	TenantID      uuid.UUID `json:"tenantId" validate:"required"`
}
