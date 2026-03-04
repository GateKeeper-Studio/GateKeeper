package githubcallback

import (
	"context"
	"encoding/json"
	"os"
	"strconv"
	"strings"

	"github.com/gate-keeper/internal/domain/entities"
	"github.com/gate-keeper/internal/domain/errors"
	mfa_policy "github.com/gate-keeper/internal/domain/services"
	application_utils "github.com/gate-keeper/internal/features/utils"
	"github.com/gate-keeper/internal/infra/database/repositories"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	mailservice "github.com/gate-keeper/internal/infra/mail-service"
)

type Handler struct {
	repository  IRepository
	mailService mailservice.IMailService
}

func New(q *pgstore.Queries) repositories.ServiceHandlerRs[Command, *ServiceResponse] {
	return &Handler{
		repository:  NewRepository(q),
		mailService: &mailservice.MailService{},
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
			oauthProvider.ApplicationID,
			false,
		)

		if err != nil {
			return nil, err
		}

		// GitHub has verified the user's email address.
		newUser.IsEmailConfirmed = true

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

	// Confirm email for existing users authenticated via GitHub, since GitHub verifies email ownership.
	if !currentUser.IsEmailConfirmed {
		currentUser.IsEmailConfirmed = true
		if _, err := s.repository.UpdateUser(ctx, currentUser); err != nil {
			return nil, err
		}
	}

	// --- Adaptive MFA Policy Evaluation ---
	// GitHub does not support OIDC amr claims, so AMR is always empty.
	// The policy engine will evaluate based on application settings and user MFA config.
	application, err := s.repository.GetApplicationByID(ctx, oauthProvider.ApplicationID)
	if err != nil {
		return nil, err
	}

	policyDecision := mfa_policy.EvaluateMfaRequirement(mfa_policy.MfaPolicyContext{
		AuthProvider: "github",
		AmrClaims:    nil, // GitHub does not provide AMR claims
		User:         currentUser,
		Application:  application,
	})

	if policyDecision.RequiresMfa {
		// MFA is required — create the appropriate challenge.
		mfaChallenge, err := application_utils.CreateMfaChallenge(
			ctx,
			s.repository,
			s.mailService,
			currentUser,
			oauthProvider.ApplicationID,
		)
		if err != nil {
			return nil, err
		}
		if mfaChallenge != nil {
			return &ServiceResponse{
				RedirectURL:               os.Getenv("CLIENT_APPLICATION_URL") + "/api/callback/github",
				UserData:                  &gitHubUserData,
				OauthProviderID:           externalOauthState.ApplicationOAuthProviderID,
				ClientState:               externalOauthState.ClientState,
				ClientCodeChallengeMethod: externalOauthState.ClientCodeChallengeMethod,
				ClientCodeChallenge:       externalOauthState.ClientCodeChallenge,
				ClientScope:               externalOauthState.ClientScope,
				ClientResponseType:        externalOauthState.ClientResponseType,
				ClientRedirectUri:         externalOauthState.ClientRedirectUri,
				ClientNonce:               externalOauthState.ClientNonce,
				MfaRequired:               true,
				MfaChallenge:              mfaChallenge,
			}, nil
		}
	}

	// No MFA required — create authorization code directly.
	authorizationCode, err := entities.CreateApplicationAuthorizationCode(
		oauthProvider.ApplicationID,
		currentUser.ID,
		externalOauthState.ClientRedirectUri,
		externalOauthState.ClientCodeChallenge,
		externalOauthState.ClientCodeChallengeMethod,
		externalOauthState.ClientNonce,
		&externalOauthState.ClientScope,
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
		RedirectURL:               os.Getenv("CLIENT_APPLICATION_URL") + "/api/callback/github",
		UserData:                  &gitHubUserData,
		OauthProviderID:           externalOauthState.ApplicationOAuthProviderID,
		ClientState:               externalOauthState.ClientState,
		AuthorizationCode:         authorizationCode.ID.String(),
		ClientCodeChallengeMethod: externalOauthState.ClientCodeChallengeMethod,
		ClientCodeChallenge:       externalOauthState.ClientCodeChallenge,
		ClientScope:               externalOauthState.ClientScope,
		ClientResponseType:        externalOauthState.ClientResponseType,
		ClientRedirectUri:         externalOauthState.ClientRedirectUri,
		ClientNonce:               externalOauthState.ClientNonce,
	}, nil
}
