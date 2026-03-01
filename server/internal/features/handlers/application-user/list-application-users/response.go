package listapplicationusers

import "github.com/google/uuid"

type UserRole struct {
	ID          uuid.UUID `json:"id"`
	Name        string    `json:"name"`
	Description *string   `json:"description"`
}

type UserResponse struct {
	ID          uuid.UUID  `json:"id"`
	DisplayName string     `json:"displayName"`
	Email       string     `json:"email"`
	Roles       []UserRole `json:"roles"`
}

type Response struct {
	TotalCount int            `json:"totalCount"`
	Page       int            `json:"page"`
	PageSize   int            `json:"pageSize"`
	Data       []UserResponse `json:"data"`
}
