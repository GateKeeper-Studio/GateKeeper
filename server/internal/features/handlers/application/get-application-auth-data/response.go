package getapplicationauthdata

import "github.com/google/uuid"

type Response struct {
	ID                uuid.UUID              `json:"id"`
	Name              string                 `json:"name"`
	CanSelfSignUp     bool                   `json:"canSelfSignUp"`
	CanSelfForgotPass bool                   `json:"canSelfForgotPass"`
	OAuthProviders    []ApplicationProviders `json:"oauthProviders"`
}

type ApplicationProviders struct {
	ID   uuid.UUID `json:"id"`
	Name string    `json:"name"`
}
