package githubcallback

import (
	"context"
	"encoding/json"
	"log"

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
	if request.Code == "" {
		return nil, &errors.ErrInvalidOAuthCode
	}

	external_oauth_state, err := s.repository.GetExternalOAuthStateByState(ctx, request.State)

	if err != nil {
		return nil, err
	}

	if external_oauth_state == nil {
		return nil, &errors.ErrInvalidOAuthState
	}

	oauthProvider, err := s.repository.GetApplicationOAuthProviderByID(ctx, external_oauth_state.ApplicationOAuthProviderID)

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
				"Accept":       "application/json",
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
		log.Println("Error decoding GitHub response: ", err)
		return nil, err
	}

	resp, err := application_utils.Fetch(
		"GET",
		"https://api.github.com/user",
		&application_utils.FetchOptions{
			Headers: map[string]string{
				"Authorization":        "Bearer " + githubResponsePayloadObj.AccessToken,
				"X-GitHub-Api-Version": "2022-11-28",
			},
		},
	)

	if err != nil {
		return nil, err
	}

	defer resp.Body.Close()

	var gitHubUserData GitHubUserData

	if err := json.NewDecoder(resp.Body).Decode(&gitHubUserData); err != nil {
		return nil, err
	}

	return &ServiceResponse{
		RedirectURL:               "http://localhost:3001/api/callback/github",
		UserData:                  &gitHubUserData,
		OauthProviderID:           external_oauth_state.ApplicationOAuthProviderID,
		ClientState:               external_oauth_state.ClientState,
		ClientCodeChallengeMethod: external_oauth_state.ClientCodeChallengeMethod,
		ClientCodeChallenge:       external_oauth_state.ClientCodeChallenge,
		ClientScope:               external_oauth_state.ClientScope,
		ClientResponseType:        external_oauth_state.ClientResponseType,
		ClientRedirectUri:         external_oauth_state.ClientRedirectUri,
	}, nil
}
