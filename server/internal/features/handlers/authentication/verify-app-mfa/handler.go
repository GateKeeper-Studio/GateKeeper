package verifyappmfa

import (
	"context"

	"github.com/gate-keeper/internal/domain/constants"
	"github.com/gate-keeper/internal/domain/entities"
	"github.com/gate-keeper/internal/domain/errors"
	"github.com/gate-keeper/internal/infra/database/repositories"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	mailservice "github.com/gate-keeper/internal/infra/mail-service"
	"github.com/pquerna/otp/totp"
)

type Handler struct {
	repository  IRepository
	mailService mailservice.IMailService
}

func New(q *pgstore.Queries) repositories.ServiceHandlerRs[Command, *Response] {
	return &Handler{
		repository:  Repository{Store: q},
		mailService: &mailservice.MailService{},
	}
}

func (s *Handler) Handler(ctx context.Context, command Command) (*Response, error) {
	user, err := s.repository.GetUserByEmail(ctx, command.Email, command.ApplicationID)

	if err != nil {
		return nil, err
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

	mfaMethod, err := s.repository.GetMfaMethodByUserIDAndMethod(ctx, user.ID, constants.MfaMethodTotp)

	if err != nil {
		return nil, err
	}

	if !mfaMethod.Enabled {
		return nil, &errors.ErrMfaAppNotEnabled
	}

	mfaTotpCode, err := s.repository.GetMfaTotpCodeByID(ctx, *command.MfaID)

	if err != nil {
		return nil, &errors.ErrAppMfaCodeNotFound
	}

	if mfaTotpCode == nil {
		return nil, &errors.ErrAppMfaCodeNotFound
	}

	if err := s.repository.DeleteMfaTotpCode(ctx, mfaTotpCode.ID); err != nil {
		return nil, err
	}

	isValid := totp.Validate(command.Code, mfaTotpCode.Secret)

	if !isValid {
		return nil, &errors.ErrInvalidMfaAuthAppCode
	}

	authorizationSession, err := entities.CreateSessionCode(user.ID, command.ApplicationID)

	if err != nil {
		return nil, err
	}

	if err := s.repository.AddAuthorizationSession(ctx, authorizationSession); err != nil {
		return nil, err
	}

	return &Response{
		SessionCode: authorizationSession.Token,
	}, nil
}
