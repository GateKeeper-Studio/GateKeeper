package authorize

import (
	"context"
	"time"

	"github.com/gate-keeper/internal/domain/constants"
	"github.com/gate-keeper/internal/domain/entities"
	"github.com/gate-keeper/internal/domain/errors"
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
	user, err := s.repository.GetUserByEmail(ctx, command.Email, command.ApplicationID)

	if err != nil {
		return nil, &errors.ErrUserNotFound
	}

	if user == nil {
		return nil, &errors.ErrUserNotFound
	}

	if !user.IsActive {
		return nil, &errors.ErrUserNotActive
	}

	if !user.IsEmailConfirmed {
		return nil, &errors.ErrEmailNotConfirmed
	}

	userCredentials, err := s.repository.GetUserCredentialsByUserID(ctx, user.ID)

	if err != nil {
		return nil, err
	}

	if userCredentials.ShouldChangePass {
		return nil, &errors.ErrUserShouldChangePassword
	}

	sessionCode, err := s.repository.GetAuthorizationSession(ctx, user.ID, command.SessionCode)

	if err != nil {
		return nil, err
	}

	if sessionCode == nil {
		return nil, &errors.ErrSessionCodeNotFound
	}

	if sessionCode.ExpiresAt.Before(time.Now().UTC()) {
		return nil, &errors.ErrSessionCodeExpired
	}

	if user.Preferred2FAMethod != nil && *user.Preferred2FAMethod == constants.MfaMethodTotp {
		if command.MfaID == nil {
			return nil, &errors.ErrMfaCodeRequired
		}

		mfaTotpCode, err := s.repository.GetMfaTotpCodeByID(ctx, *command.MfaID)

		if err != nil {
			return nil, err
		}

		if mfaTotpCode == nil {
			return nil, &errors.ErrMfaCodeNotFound
		}

		// Delete the MFA app code after successful authorization
		if err := s.repository.DeleteMfaTotpCodeByID(ctx, user.ID); err != nil {
			return nil, err
		}
	}

	s.repository.DeleteSessionCodeByID(ctx, sessionCode.ID)

	authorizationCode, err := entities.CreateApplicationAuthorizationCode(
		command.ApplicationID,
		user.ID,
		command.RedirectUri,
		command.CodeChallenge,
		command.CodeChallengeMethod,
	)

	if err != nil {
		return nil, err
	}

	if err := s.repository.RemoveAuthorizationCode(ctx, user.ID, command.ApplicationID); err != nil {
		return nil, err
	}

	if err := s.repository.AddAuthorizationCode(ctx, authorizationCode); err != nil {
		return nil, err
	}

	return &Response{
		AuthorizationCode:   authorizationCode.ID.String(),
		RedirectUri:         command.RedirectUri,
		State:               command.State,
		CodeChallenge:       command.CodeChallenge,
		CodeChallengeMethod: command.CodeChallengeMethod,
		ResponseType:        command.ResponseType,
	}, nil
}
