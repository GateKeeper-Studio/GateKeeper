package gettenantuser

import (
	"github.com/google/uuid"
)

type Response struct {
	ID                     uuid.UUID          `json:"id"`
	Email                  string             `json:"email"`
	TenantID               uuid.UUID          `json:"tenantId"`
	TenantName             string             `json:"tenantName"`
	DisplayName            string             `json:"displayName"`
	IsActive               bool               `json:"isActive"`
	FirstName              string             `json:"firstName"`
	Lastname               string             `json:"lastName"`
	Address                *string            `json:"address"`
	PhotoURL               *string            `json:"photoUrl"`
	IsMfaEmailConfigured   bool               `json:"isMfaEmailConfigured"`
	IsMfaAuthAppConfigured bool               `json:"isMfaAuthAppConfigured"`
	Preferred2FAMethod     *string            `json:"preferred2FAMethod"`
	IsEmailVerified        bool               `json:"isEmailVerified"`
	Badges                 []UserRoleResponse `json:"badges"`
}

type UserRoleResponse struct {
	ID   uuid.UUID `json:"id"`
	Name string    `json:"name"`
}
