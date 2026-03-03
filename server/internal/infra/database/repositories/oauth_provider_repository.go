package repositories

import (
	"context"

	"github.com/gate-keeper/internal/domain/entities"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	"github.com/google/uuid"
	"github.com/jackc/pgx/v5/pgtype"
)

// IOAuthProviderRepository defines all operations related to the ApplicationOAuthProvider and ExternalOAuthState entities.
type IOAuthProviderRepository interface {
	GetApplicationOAuthProviderByID(ctx context.Context, applicationOauthProviderID uuid.UUID) (*entities.ApplicationOAuthProvider, error)
	AddApplicationOauthProvider(ctx context.Context, applicationOauthProvider *entities.ApplicationOAuthProvider) error
	UpdateApplicationOauthProvider(ctx context.Context, applicationOauthProvider *entities.ApplicationOAuthProvider) error
	GetApplicationOauthProviderByName(ctx context.Context, applicationID uuid.UUID, name string) (*entities.ApplicationOAuthProvider, error)
	CheckIfApplicationOauthProviderConfigurationExists(ctx context.Context, applicationID uuid.UUID, name string) (bool, error)
	GetApplicationOAuthProviders(ctx context.Context, applicationID uuid.UUID) (*[]entities.ApplicationOAuthProvider, error)
	AddExternalOAuthState(ctx context.Context, state *entities.ExternalOAuthState) error
	GetExternalOAuthStateByState(ctx context.Context, providerState string) (*entities.ExternalOAuthState, error)
}

// OAuthProviderRepository is the shared implementation for OAuthProvider-related DB operations.
type OAuthProviderRepository struct {
	Store *pgstore.Queries
}

func (r OAuthProviderRepository) GetApplicationOAuthProviderByID(ctx context.Context, applicationOauthProviderID uuid.UUID) (*entities.ApplicationOAuthProvider, error) {
	oauthProvider, err := r.Store.GetApplicationOauthProviderByID(ctx, applicationOauthProviderID)

	if err == ErrNoRows {
		return nil, nil
	}

	if err != nil {
		return nil, err
	}

	return &entities.ApplicationOAuthProvider{
		ID:            oauthProvider.ID,
		Name:          oauthProvider.Name,
		Enabled:       oauthProvider.Enabled,
		ApplicationID: oauthProvider.ApplicationID,
		CreatedAt:     oauthProvider.CreatedAt.Time,
		UpdatedAt:     oauthProvider.UpdatedAt,
		ClientID:      oauthProvider.ClientID,
		ClientSecret:  oauthProvider.ClientSecret,
		RedirectURI:   oauthProvider.RedirectUri,
	}, nil
}

func (r OAuthProviderRepository) AddApplicationOauthProvider(ctx context.Context, applicationOauthProvider *entities.ApplicationOAuthProvider) error {
	return r.Store.AddApplicationOauthProvider(ctx, pgstore.AddApplicationOauthProviderParams{
		ID:            applicationOauthProvider.ID,
		ApplicationID: applicationOauthProvider.ApplicationID,
		Name:          applicationOauthProvider.Name,
		ClientID:      applicationOauthProvider.ClientID,
		ClientSecret:  applicationOauthProvider.ClientSecret,
		RedirectUri:   applicationOauthProvider.RedirectURI,
		CreatedAt:     pgtype.Timestamp{Time: applicationOauthProvider.CreatedAt, Valid: true},
		UpdatedAt:     applicationOauthProvider.UpdatedAt,
		Enabled:       applicationOauthProvider.Enabled,
	})
}

func (r OAuthProviderRepository) UpdateApplicationOauthProvider(ctx context.Context, applicationOauthProvider *entities.ApplicationOAuthProvider) error {
	return r.Store.UpdateApplicationOauthProvider(ctx, pgstore.UpdateApplicationOauthProviderParams{
		ID:           applicationOauthProvider.ID,
		Name:         applicationOauthProvider.Name,
		ClientID:     applicationOauthProvider.ClientID,
		ClientSecret: applicationOauthProvider.ClientSecret,
		RedirectUri:  applicationOauthProvider.RedirectURI,
		UpdatedAt:    applicationOauthProvider.UpdatedAt,
		Enabled:      applicationOauthProvider.Enabled,
	})
}

func (r OAuthProviderRepository) GetApplicationOauthProviderByName(ctx context.Context, applicationID uuid.UUID, name string) (*entities.ApplicationOAuthProvider, error) {
	applicationOauthProvider, err := r.Store.GetApplicationOauthProviderByName(ctx, pgstore.GetApplicationOauthProviderByNameParams{
		ApplicationID: applicationID,
		Name:          name,
	})

	if err == ErrNoRows {
		return nil, nil
	}

	if err != nil {
		return nil, err
	}

	return &entities.ApplicationOAuthProvider{
		ID:            applicationOauthProvider.ID,
		ApplicationID: applicationOauthProvider.ApplicationID,
		Name:          applicationOauthProvider.Name,
		ClientID:      applicationOauthProvider.ClientID,
		ClientSecret:  applicationOauthProvider.ClientSecret,
		RedirectURI:   applicationOauthProvider.RedirectUri,
		CreatedAt:     applicationOauthProvider.CreatedAt.Time,
		UpdatedAt:     applicationOauthProvider.UpdatedAt,
		Enabled:       applicationOauthProvider.Enabled,
	}, nil
}

func (r OAuthProviderRepository) CheckIfApplicationOauthProviderConfigurationExists(ctx context.Context, applicationID uuid.UUID, name string) (bool, error) {
	exists, err := r.Store.CheckIfApplicationOauthProviderConfigurationExists(ctx, pgstore.CheckIfApplicationOauthProviderConfigurationExistsParams{
		ApplicationID: applicationID,
		Name:          name,
	})

	if err != nil {
		return false, err
	}

	return exists, nil
}

func (r OAuthProviderRepository) GetApplicationOAuthProviders(ctx context.Context, applicationID uuid.UUID) (*[]entities.ApplicationOAuthProvider, error) {
	oauthProviders, err := r.Store.GetApplicationOauthProvidersByApplicationID(ctx, applicationID)

	if err == ErrNoRows {
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

func (r OAuthProviderRepository) AddExternalOAuthState(ctx context.Context, state *entities.ExternalOAuthState) error {
	codeVerifier := ""
	if state.ClientCodeVerifier != nil {
		codeVerifier = *state.ClientCodeVerifier
	}

	return r.Store.AddExternalOAuthState(ctx, pgstore.AddExternalOAuthStateParams{
		ID:                         state.ID,
		ProviderState:              state.ProviderState,
		ApplicationOauthProviderID: state.ApplicationOAuthProviderID,
		ClientState:                &state.ClientState,
		ClientCodeChallengeMethod:  &state.ClientCodeChallengeMethod,
		ClientCodeChallenge:        &state.ClientCodeChallenge,
		ClientScope:                &state.ClientScope,
		ClientResponseType:         &state.ClientResponseType,
		ClientRedirectUri:          &state.ClientRedirectUri,
		CodeVerifier:               codeVerifier,
		ClientNonce:                state.ClientNonce,
		CreatedAt:                  pgtype.Timestamp{Time: state.CreatedAt, Valid: true},
	})
}

func (r OAuthProviderRepository) GetExternalOAuthStateByState(ctx context.Context, providerState string) (*entities.ExternalOAuthState, error) {
	oauthState, err := r.Store.GetExternalOAuthStateByState(ctx, providerState)
	if err != nil {
		return nil, err
	}

	return &entities.ExternalOAuthState{
		ID:                         oauthState.ID,
		ProviderState:              oauthState.ProviderState,
		ClientState:                *oauthState.ClientState,
		ClientCodeChallengeMethod:  *oauthState.ClientCodeChallengeMethod,
		ClientCodeChallenge:        *oauthState.ClientCodeChallenge,
		ClientScope:                *oauthState.ClientScope,
		ClientResponseType:         *oauthState.ClientResponseType,
		ClientCodeVerifier:         &oauthState.CodeVerifier,
		ClientRedirectUri:          *oauthState.ClientRedirectUri,
		ClientNonce:                oauthState.ClientNonce,
		ApplicationOAuthProviderID: oauthState.ApplicationOauthProviderID,
		CreatedAt:                  oauthState.CreatedAt.Time,
	}, nil
}
