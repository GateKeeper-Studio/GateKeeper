package googlelogin

import (
	"context"

	"github.com/gate-keeper/internal/domain/entities"
	"github.com/gate-keeper/internal/domain/errors"
	application_utils "github.com/gate-keeper/internal/features/utils"
	"github.com/gate-keeper/internal/infra/database/repositories"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
)

type Handler struct {
	repository IRepository
}

func New(q *pgstore.Queries) repositories.ServiceHandlerRs[Command, *ServiceResponse] {
	return &Handler{
		repository: Repository{Store: q},
	}
}

func (s *Handler) Handler(ctx context.Context, request Command) (*ServiceResponse, error) {
	state := entities.GenerateRandomString(32)
	scope := "openid email profile"

	oauthProvider, err := s.repository.GetApplicationOAuthProviderByID(ctx, request.OauthProviderId)

	if err != nil {
		return nil, err
	}

	if oauthProvider == nil {
		return nil, &errors.ErrOAuthProviderNotFound
	}

	codeVerifier, err := application_utils.GenerateCodeVerifier()

	if err != nil {
		return nil, err
	}

	codeChallenge := application_utils.GenerateCodeChallenge(codeVerifier, request.ClientCodeChallengeMethod)

	externalOauthState := entities.AddExternalOAuthState(
		state,
		oauthProvider.ID,
		request.ClientState,
		request.ClientCodeChallengeMethod,
		request.ClientCodeChallenge,
		&codeVerifier,
		request.ClientScope,
		request.ClientResponseType,
		request.ClientRedirectUri,
	)

	if err := s.repository.AddExternalOAuthState(ctx, externalOauthState); err != nil {
		return nil, err
	}

	return &ServiceResponse{
		State:               state,
		ClientID:            oauthProvider.ClientID,
		RedirectURI:         oauthProvider.RedirectURI,
		Scope:               scope,
		CodeChallenge:       codeChallenge,
		CodeChallengeMethod: request.ClientCodeChallengeMethod,
	}, nil
}
