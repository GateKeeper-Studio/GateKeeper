package entities

import (
	"time"

	"github.com/google/uuid"
)

const (
	OAuthProviderNameGoogle = "google"
	OAuthProviderNameGitHub = "github"
)

type ApplicationOAuthProvider struct {
	ID            uuid.UUID
	ApplicationID uuid.UUID
	Name          string
	Enabled       bool
	ClientID      string
	ClientSecret  string
	RedirectURI   string
	CreatedAt     time.Time
	UpdatedAt     *time.Time
}

func VerifyOAuthProviderName(name string) bool {
	switch name {
	case OAuthProviderNameGoogle, OAuthProviderNameGitHub:
		return true
	default:
		return false
	}
}

func NewApplicationOAuthProvider(applicationID uuid.UUID, name string, clientID string, clientSecret string, redirectURI string, enabled bool) *ApplicationOAuthProvider {
	newID, err := uuid.NewV7()

	if err != nil {
		panic(err)
	}

	if !VerifyOAuthProviderName(name) {
		panic("Invalid OAuth provider name")
	}

	return &ApplicationOAuthProvider{
		ID:            newID,
		ApplicationID: applicationID,
		Name:          name,
		ClientID:      clientID,
		ClientSecret:  clientSecret,
		RedirectURI:   redirectURI,
		Enabled:       enabled,
		CreatedAt:     time.Now(),
		UpdatedAt:     nil,
	}
}
