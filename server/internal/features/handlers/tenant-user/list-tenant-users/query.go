package listtenantusers

import "github.com/google/uuid"

type Query struct {
	TenantID uuid.UUID `json:"tenantId" validate:"required,uuid"`
	Page     int       `json:"page"`
	PageSize int       `json:"pageSize"`
}
