package githubcallback

import (
	"context"
	"encoding/json"
	"strconv"
	"strings"

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
	if request.Code == "" {
		return nil, &errors.ErrInvalidOAuthCode
	}

	externalOauthState, err := s.repository.GetExternalOAuthStateByState(ctx, request.State)

	if err != nil {
		return nil, err
	}

	if externalOauthState == nil {
		return nil, &errors.ErrInvalidOAuthState
	}

	oauthProvider, err := s.repository.GetApplicationOAuthProviderByID(ctx, externalOauthState.ApplicationOAuthProviderID)

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
				Code:         request.Code, // From the callback request
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
		return nil, err
	}

	emailsResp, err := application_utils.Fetch(
		"GET",
		"https://api.github.com/user/emails",
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

	defer emailsResp.Body.Close()

	var emails []struct {
		Email    string `json:"email"`
		Primary  bool   `json:"primary"`
		Verified bool   `json:"verified"`
	}

	if err := json.NewDecoder(emailsResp.Body).Decode(&emails); err != nil {
		return nil, err
	}

	var primaryEmail string

	for _, e := range emails {
		if e.Primary && e.Verified {
			primaryEmail = e.Email
			break
		}
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

	gitHubUserData.Email = strings.ToLower(primaryEmail) // Set the primary email

	currentUser, err := s.repository.GetUserByEmail(ctx, gitHubUserData.Email, oauthProvider.ApplicationID)

	if err != nil {
		return nil, err
	}

	// externalProviderKey := entities.OAuthProviderNameGitHub

	if currentUser == nil {
		newUser, err := entities.CreateApplicationUser(
			gitHubUserData.Email,
			nil,
			oauthProvider.ApplicationID,
			false,
		)

		if err != nil {
			return nil, err
		}

		if err = s.repository.AddUser(ctx, newUser); err != nil {
			return nil, err
		}

		err = s.repository.AddUserProfile(ctx, &entities.UserProfile{
			UserID:      newUser.ID,
			DisplayName: gitHubUserData.Name,
			FirstName:   strings.Split(gitHubUserData.Name, " ")[0],
			LastName:    strings.Join(strings.Split(gitHubUserData.Name, " ")[1:], " "),
			PhoneNumber: nil,
			Address:     nil,
			PhotoURL:    &gitHubUserData.AvatarURL,
		})

		if err != nil {
			return nil, err
		}

		externalIdentity := entities.CreateExternalIdentity(
			newUser.ID,
			gitHubUserData.Email,
			entities.OAuthProviderNameGitHub,
			strconv.Itoa(gitHubUserData.ID),
			oauthProvider.ID,
		)

		if err := s.repository.AddExternalIdentity(ctx, externalIdentity); err != nil {
			return nil, err
		}

		currentUser = newUser
	}

	authorizationCode, err := entities.CreateApplicationAuthorizationCode(
		oauthProvider.ApplicationID,
		currentUser.ID,
		externalOauthState.ClientRedirectUri,
		externalOauthState.ClientCodeChallenge,
		externalOauthState.ClientCodeChallengeMethod,
	)

	if err != nil {
		return nil, err
	}

	if err := s.repository.RemoveAuthorizationCode(ctx, currentUser.ID, oauthProvider.ApplicationID); err != nil {
		return nil, err
	}

	if err := s.repository.AddAuthorizationCode(ctx, authorizationCode); err != nil {
		return nil, err
	}

	return &ServiceResponse{
		RedirectURL:               "http://localhost:3001/api/callback/github",
		UserData:                  &gitHubUserData,
		OauthProviderID:           externalOauthState.ApplicationOAuthProviderID,
		ClientState:               externalOauthState.ClientState,
		AuthorizationCode:         authorizationCode.ID.String(),
		ClientCodeChallengeMethod: externalOauthState.ClientCodeChallengeMethod,
		ClientCodeChallenge:       externalOauthState.ClientCodeChallenge,
		ClientScope:               externalOauthState.ClientScope,
		ClientResponseType:        externalOauthState.ClientResponseType,
		ClientRedirectUri:         externalOauthState.ClientRedirectUri,
	}, nil
}
