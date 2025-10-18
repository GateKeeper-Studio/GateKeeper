package githublogin

import (
	"context"

	"github.com/gate-keeper/internal/domain/entities"
	"github.com/gate-keeper/internal/infra/database/repositories"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	"github.com/google/uuid"
)

type IRepository interface {
	GetApplicationOAuthProviderByID(ctx context.Context, applicationOauthProviderID uuid.UUID) (*entities.ApplicationOAuthProvider, error)
}

type Repository struct {
	Store *pgstore.Queries
}

func (r Repository) GetApplicationOAuthProviderByID(ctx context.Context, applicationOauthProviderID uuid.UUID) (*entities.ApplicationOAuthProvider, error) {
	oauthProvider, err := r.Store.GetApplicationOauthProviderByID(ctx, applicationOauthProviderID)

	if err == repositories.ErrNoRows {
		return nil, nil
	}

	if err != nil {
		return nil, err
	}

	oauthProviderEntity := &entities.ApplicationOAuthProvider{
		ID:            oauthProvider.ID,
		Name:          oauthProvider.Name,
		Enabled:       oauthProvider.Enabled,
		ApplicationID: oauthProvider.ApplicationID,
		CreatedAt:     oauthProvider.CreatedAt.Time,
		UpdatedAt:     oauthProvider.UpdatedAt,
		ClientID:      oauthProvider.ClientID,
		ClientSecret:  oauthProvider.ClientSecret,
		RedirectURI:   oauthProvider.RedirectUri,
	}

	return oauthProviderEntity, nil
}
