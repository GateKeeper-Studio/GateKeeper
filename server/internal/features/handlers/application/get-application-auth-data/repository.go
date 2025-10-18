package getapplicationauthdata

import (
	"context"
	"strings"

	"github.com/gate-keeper/internal/domain/entities"
	"github.com/gate-keeper/internal/infra/database/repositories"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	"github.com/google/uuid"
)

type IRepository interface {
	GetApplicationByID(ctx context.Context, applicationID uuid.UUID) (*entities.Application, error)
	GetApplicationOAuthProviders(ctx context.Context, applicationID uuid.UUID) (*[]entities.ApplicationOAuthProvider, error)
}

type Repository struct {
	Store *pgstore.Queries
}

func (r Repository) GetApplicationOAuthProviders(ctx context.Context, applicationID uuid.UUID) (*[]entities.ApplicationOAuthProvider, error) {
	oauthProviders, err := r.Store.GetApplicationOauthProvidersByApplicationID(ctx, applicationID)

	if err == repositories.ErrNoRows {
		return nil, nil
	}

	if err != nil {
		return nil, err
	}

	oauthProvidersEntities := make([]entities.ApplicationOAuthProvider, len(oauthProviders))

	for i, provider := range oauthProviders {
		oauthProvidersEntities[i] = entities.ApplicationOAuthProvider{
			ID:            provider.ID,
			Name:          provider.Name,
			Enabled:       provider.Enabled,
			ApplicationID: provider.ApplicationID,
			CreatedAt:     provider.CreatedAt.Time,
			UpdatedAt:     provider.UpdatedAt,
			ClientID:      provider.ClientID,
			ClientSecret:  provider.ClientSecret,
			RedirectURI:   provider.RedirectUri,
		}
	}

	return &oauthProvidersEntities, nil
}

func (r Repository) GetApplicationByID(ctx context.Context, applicationID uuid.UUID) (*entities.Application, error) {
	application, err := r.Store.GetApplicationByID(ctx, applicationID)

	if err == repositories.ErrNoRows {
		return nil, nil
	}

	if err != nil {
		return nil, err
	}

	return &entities.Application{
		ID:                 application.ID,
		Name:               application.Name,
		Description:        application.Description,
		OrganizationID:     application.OrganizationID,
		CreatedAt:          application.CreatedAt.Time,
		IsActive:           application.IsActive,
		HasMfaAuthApp:      application.HasMfaAuthApp,
		HasMfaEmail:        application.HasMfaEmail,
		PasswordHashSecret: application.PasswordHashSecret,
		UpdatedAt:          application.UpdatedAt,
		Badges:             strings.Split(*application.Badges, ","),
		CanSelfSignUp:      application.CanSelfSignUp,
		CanSelfForgotPass:  application.CanSelfForgotPass,
	}, nil
}
