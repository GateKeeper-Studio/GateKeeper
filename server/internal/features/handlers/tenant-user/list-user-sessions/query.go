package listusersessions

import "github.com/google/uuid"

type Query struct {
	TenantID uuid.UUID `json:"tenantId" validate:"required,uuid"`
	UserID   uuid.UUID `json:"userId" validate:"required,uuid"`
}
