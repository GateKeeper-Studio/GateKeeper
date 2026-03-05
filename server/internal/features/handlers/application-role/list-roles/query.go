package listroles

import "github.com/google/uuid"

type Query struct {
	ApplicationID uuid.UUID `json:"applicationId" validate:"required,uuid"`
	TenantID      uuid.UUID `json:"tenantId" validate:"required,uuid"`
	Page          int       `json:"page"`
	PageSize      int       `json:"pageSize"`
}
