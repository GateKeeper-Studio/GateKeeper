package signincredential

import (
	"context"
	"log/slog"
	"strings"

	"github.com/gate-keeper/internal/domain/errors"
	application_utils "github.com/gate-keeper/internal/features/utils"
	"github.com/gate-keeper/internal/infra/database/repositories"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
)

type Handler struct {
	repository IRepository
}

func New(q *pgstore.Queries) repositories.ServiceHandlerRs[Command, *Response] {
	return &Handler{
		repository: NewRepository(q),
	}
}

func (s *Handler) Handler(ctx context.Context, command Command) (*Response, error) {
	authorizationCode, err := handleAuthorizationCode(ctx, s, command)

	if err != nil {
		return nil, err
	}

	// Verify CodeChallenge from authorization code (PKCE RFC 7636)
	isCodeChallengeValid, err := application_utils.VerifyCodeChallenge(
		command.CodeVerifier,
		authorizationCode.CodeChallenge,
		authorizationCode.CodeChallengeMethod,
	)

	if err != nil {
		return nil, err
	}

	if !isCodeChallengeValid {
		return nil, &errors.ErrInvalidCodeChallenge
	}

	secrets, err := s.repository.ListSecretsFromApplication(ctx, authorizationCode.ApplicationID)

	if err != nil {
		return nil, err
	}

	isClientSecretValid, err := application_utils.VerifyClientSecret(command.ClientSecret, secrets)

	if err != nil {
		return nil, err
	}

	if !isClientSecretValid {
		return nil, &errors.ErrInvalidClientSecret
	}

	if err := s.repository.RemoveAuthorizationCode(ctx, authorizationCode.ApplicationUserId, authorizationCode.ApplicationID); err != nil {
		return nil, err
	}

	application, err := s.repository.GetApplicationByID(ctx, authorizationCode.ApplicationID)
	if err != nil || application == nil {
		return nil, &errors.ErrApplicationNotFound
	}

	user, err := s.repository.GetUserByID(ctx, authorizationCode.ApplicationUserId)

	if err != nil {
		return nil, err
	}

	if user == nil {
		return nil, &errors.ErrUserNotFound
	}

	refreshToken, err := assignRefreshToken(ctx, s, *user, application.RefreshTokenTTLDays)

	if err != nil {
		return nil, err
	}

	userProfile, err := s.repository.GetUserProfileByID(ctx, user.ID)

	if err != nil {
		return nil, err
	}

	jwtClaims := application_utils.JWTClaims{
		UserID:        user.ID,
		Email:         user.Email,
		FirstName:     userProfile.FirstName,
		LastName:      userProfile.LastName,
		DisplayName:   userProfile.DisplayName,
		ApplicationID: user.ApplicationID,
	}

	jwtToken, err := application_utils.CreateToken(jwtClaims)

	if err != nil {
		return nil, err
	}

	// Determine scope and issue OIDC ID Token when openid scope was requested
	scope := "openid profile email"
	if authorizationCode.Scope != nil && *authorizationCode.Scope != "" {
		scope = *authorizationCode.Scope
	}

	var idToken string
	if strings.Contains(scope, "openid") {
		audience := command.ClientID.String()
		idToken, err = application_utils.CreateIDToken(jwtClaims, authorizationCode.Nonce, audience)
		if err != nil {
			return nil, err
		}
	}

	slog.InfoContext(ctx, "User signed in successfully")

	return &Response{
		User: UserResponse{
			ID:            user.ID,
			DisplayName:   userProfile.DisplayName,
			FirstName:     userProfile.FirstName,
			LastName:      userProfile.LastName,
			Email:         user.Email,
			PhotoURL:      userProfile.PhotoURL,
			CreatedAt:     user.CreatedAt,
			ApplicationID: user.ApplicationID,
		},
		AccessToken:  jwtToken,
		IDToken:      idToken,
		RefreshToken: refreshToken.ID,
		TokenType:    "Bearer",
		ExpiresIn:    900, // 15 minutes in seconds
		Scope:        scope,
	}, nil
}
