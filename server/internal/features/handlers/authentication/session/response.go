package session

import "github.com/google/uuid"

type Response struct {
	User        UserData `json:"user"`
	AccessToken string   `json:"accessToken"`
}

type UserData struct {
	ID          uuid.UUID `json:"id"`
	DisplayName string    `json:"displayName"`
	TenantID    uuid.UUID `json:"tenantId"`
	FirstName   string    `json:"firstName"`
	LastName    string    `json:"lastName"`
	Email       string    `json:"email"`
	PhotoURL    *string   `json:"photoUrl"`
}
