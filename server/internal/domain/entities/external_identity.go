package entities

import (
	"time"

	"github.com/google/uuid"
)

// ExternalIdentity represents an external identity linked to an application user. E.g: GitHub, Google etc
type ExternalIdentity struct {
	ID                         uuid.UUID
	UserID                     uuid.UUID // references ApplicationUser.ID
	Email                      string    // email associated with the external identity
	Provider                   string    // e.g., "github", "google"
	ProviderUserID             string    // user ID from the external provider
	ApplicationOAuthProviderID uuid.UUID // references ApplicationOAuthProvider.ID
	CreatedAt                  time.Time
	UpdatedAt                  *time.Time
}

func CreateExternalIdentity(userID uuid.UUID, userEmail, provider, providerUserID string, applicationOAuthProviderID uuid.UUID) *ExternalIdentity {
	id, err := uuid.NewV7()

	if err != nil {
		panic("Failed to generate UUID for ExternalIdentity: " + err.Error())
	}

	return &ExternalIdentity{
		ID:                         id,
		UserID:                     userID,
		Email:                      userEmail,
		Provider:                   provider,
		ProviderUserID:             providerUserID,
		ApplicationOAuthProviderID: applicationOAuthProviderID,
		CreatedAt:                  time.Now().UTC(),
		UpdatedAt:                  nil,
	}
}
