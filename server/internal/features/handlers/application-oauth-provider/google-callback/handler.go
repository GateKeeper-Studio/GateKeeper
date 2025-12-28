package googlecallback

import (
	"context"
	"encoding/json"
	"log"
	"net/url"

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

	googleOauthProvider, err := s.repository.GetApplicationOauthProviderByName(ctx, entities.OAuthProviderNameGoogle, oauthProvider.ApplicationID)

	if err != nil {
		return nil, err
	}

	if googleOauthProvider == nil {
		return nil, &errors.ErrOAuthProviderNotFound
	}

	form := url.Values{}
	form.Set("client_id", oauthProvider.ClientID)
	form.Set("client_secret", oauthProvider.ClientSecret)
	form.Set("code", request.Code)
	form.Set("grant_type", "authorization_code")
	form.Set("redirect_uri", googleOauthProvider.RedirectURI)
	form.Set("code_verifier", *externalOauthState.ClientCodeVerifier)

	log.Println(form.Encode())

	accessTokenResp, err := application_utils.Fetch(
		"POST",
		"https://oauth2.googleapis.com/token",
		&application_utils.FetchOptions{
			Form: form,
			Headers: map[string]string{
				"Accept": "application/json",
			},
		},
	)

	if err != nil {
		return nil, err
	}

	defer accessTokenResp.Body.Close()

	var googleResponsePayloadObj googleResponsePayload

	if err := json.NewDecoder(accessTokenResp.Body).Decode(&googleResponsePayloadObj); err != nil {
		return nil, err
	}

	resp, err := application_utils.Fetch(
		"GET",
		"https://openidconnect.googleapis.com/v1/userinfo",
		&application_utils.FetchOptions{
			Headers: map[string]string{
				"Authorization": "Bearer " + googleResponsePayloadObj.AccessToken,
			},
		},
	)

	if err != nil {
		return nil, err
	}

	defer resp.Body.Close()

	var googleUserData GoogleUserData

	if err := json.NewDecoder(resp.Body).Decode(&googleUserData); err != nil {
		return nil, err
	}

	currentUser, err := s.repository.GetUserByEmail(ctx, googleUserData.Email, oauthProvider.ApplicationID)

	if err != nil {
		return nil, err
	}

	if currentUser == nil {
		newUser, err := entities.CreateApplicationUser(
			googleUserData.Email,
			oauthProvider.ApplicationID,
			false,
		)

		if err != nil {
			return nil, err
		}

		if err = s.repository.AddUser(ctx, newUser); err != nil {
			return nil, err
		}

		log.Println("Google User Data")
		log.Println(googleUserData)

		err = s.repository.AddUserProfile(ctx, &entities.UserProfile{
			UserID:      newUser.ID,
			DisplayName: googleUserData.Name,
			FirstName:   googleUserData.GivenName,
			LastName:    googleUserData.FamilyName,
			PhoneNumber: nil,
			Address:     nil,
			PhotoURL:    &googleUserData.Picture,
		})

		if err != nil {
			return nil, err
		}

		externalIdentity := entities.CreateExternalIdentity(
			newUser.ID,
			googleUserData.Email,
			entities.OAuthProviderNameGitHub,
			googleUserData.ID,
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
		RedirectURL:               "http://localhost:3001/api/callback/google",
		UserData:                  &googleUserData,
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
