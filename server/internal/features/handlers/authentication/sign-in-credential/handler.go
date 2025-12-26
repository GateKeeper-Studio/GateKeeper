package signincredential

import (
	"context"
	"log/slog"

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
		repository: Repository{Store: q},
	}
}

func (s *Handler) Handler(ctx context.Context, command Command) (*Response, error) {
	authorizationCode, err := handleAuthorizationCode(ctx, s, command)

	if err != nil {
		return nil, err
	}

	// Verify CodeChallenge from authorization code

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

	user, err := s.repository.GetUserByID(ctx, authorizationCode.ApplicationUserId)

	if err != nil {
		return nil, err
	}

	if user == nil {
		return nil, &errors.ErrUserNotFound
	}

	refreshToken, err := assignRefreshToken(ctx, s, *user)

	userProfile, err := s.repository.GetUserProfileByID(ctx, user.ID)

	if err != nil {
		return nil, err
	}

	jwtToken, err := assignTokenParams(*userProfile, *user)

	if err != nil {
		return nil, err
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
		RefreshToken: refreshToken.ID,
	}, nil
}
