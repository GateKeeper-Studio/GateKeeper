package signincredential

import (
	"time"

	"github.com/google/uuid"
)

type Response struct {
	User        UserResponse `json:"user"`
	AccessToken string       `json:"accessToken"`
	// OIDC: ID Token containing identity claims (only present when openid scope requested)
	IDToken      string    `json:"idToken,omitempty"`
	RefreshToken uuid.UUID `json:"refreshToken"`
	TokenType    string    `json:"tokenType"`
	ExpiresIn    int       `json:"expiresIn"`
	Scope        string    `json:"scope"`
}

type UserResponse struct {
	ID            uuid.UUID `json:"id"`
	DisplayName   string    `json:"displayName"`
	FirstName     string    `json:"firstName"`
	LastName      string    `json:"lastName"`
	ApplicationID uuid.UUID `json:"applicationId"`
	Email         string    `json:"email"`
	PhotoURL      *string   `json:"photoUrl"`
	CreatedAt     time.Time `json:"createdAt"`
}
