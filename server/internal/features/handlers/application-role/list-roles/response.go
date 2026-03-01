package listroles

import "github.com/google/uuid"

type RoleResponse struct {
	ID          uuid.UUID `json:"id"`
	Name        string    `json:"name"`
	Description *string   `json:"description"`
}

type Response struct {
	TotalCount int            `json:"totalCount"`
	Page       int            `json:"page"`
	PageSize   int            `json:"pageSize"`
	Data       []RoleResponse `json:"data"`
}
