package googlecallback

import (
	"context"
	"encoding/json"
	"net/url"
	"os"

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

	form := url.Values{}
	form.Set("client_id", oauthProvider.ClientID)
	form.Set("client_secret", oauthProvider.ClientSecret)
	form.Set("code", request.Code)
	form.Set("grant_type", "authorization_code")
	form.Set("redirect_uri", oauthProvider.RedirectURI)
	form.Set("code_verifier", *externalOauthState.ClientCodeVerifier)

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
		newUser, err := entities.CreateTenantUser(
			googleUserData.Email,
			oauthProvider.ApplicationID,
			false,
		)

		if err != nil {
			return nil, err
		}

		// Google has verified the user's email address.
		newUser.IsEmailConfirmed = true

		if err = s.repository.AddUser(ctx, newUser); err != nil {
			return nil, err
		}

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
			entities.OAuthProviderNameGoogle,
			googleUserData.ID,
			oauthProvider.ID,
		)

		if err := s.repository.AddExternalIdentity(ctx, externalIdentity); err != nil {
			return nil, err
		}

		currentUser = newUser
	}

	// Confirm email for existing users authenticated via Google, since Google verifies email ownership.
	if !currentUser.IsEmailConfirmed {
		currentUser.IsEmailConfirmed = true
		if _, err := s.repository.UpdateUser(ctx, currentUser); err != nil {
			return nil, err
		}
	}

	// --- Adaptive MFA Policy Evaluation ---
	// Extract AMR claims from Google's ID token (if present).
	amrClaims := application_utils.ExtractAmrFromIDToken(googleResponsePayloadObj.IdToken)

	// Fetch the application to evaluate its MFA policy settings.
	application, err := s.repository.GetApplicationByID(ctx, oauthProvider.ApplicationID)
	if err != nil {
		return nil, err
	}

	policyDecision := mfa_policy.EvaluateMfaRequirement(mfa_policy.MfaPolicyContext{
		AuthProvider: "google",
		AmrClaims:    amrClaims,
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
				RedirectURL:               os.Getenv("CLIENT_APPLICATION_URL") + "/api/callback/google",
				UserData:                  &googleUserData,
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
		RedirectURL:               os.Getenv("CLIENT_APPLICATION_URL") + "/api/callback/google",
		UserData:                  &googleUserData,
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
