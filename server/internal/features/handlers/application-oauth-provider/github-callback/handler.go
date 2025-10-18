package githubcallback

import (
	"context"
	"encoding/json"

	"github.com/gate-keeper/internal/domain/errors"
	application_utils "github.com/gate-keeper/internal/features/utils"
	"github.com/gate-keeper/internal/infra/database/repositories"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	"github.com/google/uuid"
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
	if request.State == "" || request.StoredState == "" {
		return nil, &errors.ErrInvalidOAuthState
	}

	if request.StoredProviderID == uuid.Nil {
		return nil, &errors.ErrInvalidOAuthProviderID
	}

	if request.Code == "" {
		return nil, &errors.ErrInvalidOAuthCode
	}

	oauthProvider, err := s.repository.GetApplicationOAuthProviderByID(ctx, request.StoredProviderID)

	if err != nil {
		return nil, err
	}

	if oauthProvider == nil {
		return nil, &errors.ErrOAuthProviderNotFound
	}

	accessTokenResp, err := application_utils.Fetch(
		"POST",
		"https://github.com/login/oauth/access_token",
		&application_utils.FetchOptions{
			Body: githubRequestBody{
				ClientID:     oauthProvider.ClientID,
				ClientSecret: oauthProvider.ClientSecret,
				Code:         request.Code,
			},
			Headers: map[string]string{
				"Content-Type": "application/json",
			},
		},
	)

	if err != nil {
		return nil, err
	}

	defer accessTokenResp.Body.Close()

	var githubResponsePayloadObj githubResponsePayload

	if err := json.NewDecoder(accessTokenResp.Body).Decode(&githubResponsePayloadObj); err != nil {
		return nil, err
	}

	options := &application_utils.FetchOptions{
		Headers: map[string]string{
			"Authorization": "Bearer " + githubResponsePayloadObj.AccessToken,
		},
	}

	resp, err := application_utils.Fetch(
		"POST",
		"https://api.github.com/user",
		options,
	)

	if err != nil {
		return nil, err
	}

	defer resp.Body.Close()

	return &ServiceResponse{
		RedirectURL: "http://localhost:3000/dashboard",
	}, nil
}
