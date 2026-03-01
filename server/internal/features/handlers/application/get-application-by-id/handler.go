package getapplicationbyid

import (
	"context"

	"github.com/gate-keeper/internal/domain/errors"
	"github.com/gate-keeper/internal/infra/database/repositories"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
)

type Handler struct {
	repository IRepository
}

func New(q *pgstore.Queries) repositories.ServiceHandlerRs[Query, *Response] {
	return &Handler{
		repository: Repository{Store: q},
	}
}

func (s *Handler) Handler(ctx context.Context, request Query) (*Response, error) {
	application, err := s.repository.GetApplicationByID(ctx, request.ApplicationID)

	if err != nil {
		return nil, err
	}

	if application == nil {
		return nil, &errors.ErrApplicationNotFound
	}

	secrets := make([]ApplicationSecrets, 0)
	applicationOauthProviders := make([]ApplicationProviders, 0)

	applicationSecretsDb, err := s.repository.ListSecretsFromApplication(ctx, application.ID)

	if err != nil {
		return nil, err
	}

	if applicationSecretsDb != nil {
		for _, secret := range *applicationSecretsDb {
			secrets = append(secrets, ApplicationSecrets{
				ID:             secret.ID,
				Name:           secret.Name,
				Value:          secret.Value[:len(secret.Value)/2] + "****************",
				ExpirationDate: secret.ExpiresAt,
			})
		}
	}

	applicationOauthProvidersDb, err := s.repository.GetApplicationOAuthProvidersByApplicationID(ctx, application.ID)

	if err != nil {
		return nil, err
	}

	if applicationOauthProvidersDb != nil {
		for _, provider := range *applicationOauthProvidersDb {
			applicationOauthProviders = append(applicationOauthProviders, ApplicationProviders{
				ID:           provider.ID,
				Name:         provider.Name,
				ClientID:     provider.ClientID,
				ClientSecret: provider.ClientSecret,
				RedirectURI:  provider.RedirectURI,
				UpdatedAt:    provider.UpdatedAt,
				CreatedAt:    provider.CreatedAt,
				IsEnabled:    provider.IsEnabled,
			})
		}
	}

	if len(application.Badges) == 1 && application.Badges[0] == "" {
		application.Badges = make([]string, 0)
	}

	return &Response{
		ID:                    application.ID,
		Name:                  application.Name,
		Description:           application.Description,
		Badges:                application.Badges,
		CreatedAt:             application.CreatedAt,
		UpdatedAt:             application.UpdatedAt,
		CanSelfSignUp:         application.CanSelfSignUp,
		CanSelfForgotPass:     application.CanSelfForgotPass,
		IsActive:              application.IsActive,
		MfaAuthAppEnabled:     application.HasMfaAuthApp,
		MfaEmailEnabled:       application.HasMfaEmail,
		MfaWebauthnEnabled:    application.HasMfaWebauthn,
		PasswordHashingSecret: application.PasswordHashSecret,
		Secrets:               secrets,
		OAuthProviders:        applicationOauthProviders,
	}, nil
}
