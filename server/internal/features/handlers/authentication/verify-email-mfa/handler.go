package verifyemailmfa

import (
	"context"
	"time"

	"github.com/gate-keeper/internal/domain/constants"
	"github.com/gate-keeper/internal/domain/entities"
	"github.com/gate-keeper/internal/domain/errors"
	"github.com/gate-keeper/internal/infra/database/repositories"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	mailservice "github.com/gate-keeper/internal/infra/mail-service"
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

	if *user.Preferred2FAMethod != constants.MfaMethodEmail {
		return nil, &errors.ErrMfaEmailNotEnabled
	}

	mfaMethod, err := s.repository.GetMfaMethodByUserID(ctx, user.ID, constants.MfaMethodEmail)
	if err != nil {
		return nil, err
	}

	if mfaMethod == nil || !mfaMethod.Enabled {
		return nil, &errors.ErrMfaEmailNotEnabled
	}

	emailMfaCode, err := s.repository.GetMfaEmailCodeByToken(ctx, mfaMethod.ID, command.Code)

	if err != nil {
		return nil, &errors.ErrEmailMfaCodeNotFound
	}

	if emailMfaCode == nil {
		return nil, &errors.ErrEmailMfaCodeNotFound
	}

	if emailMfaCode.ExpiresAt.Before(time.Now().UTC()) {
		return nil, &errors.ErrEmailMfaCodeExpired
	}

	s.repository.DeleteEmailMfaCodeByID(ctx, emailMfaCode.ID)

	sessionCode, err := entities.CreateSessionCode(user.ID, command.ApplicationID)

	if err != nil {
		return nil, err
	}

	if err := s.repository.AddSessionCode(ctx, sessionCode); err != nil {
		return nil, err
	}

	return &Response{
		SessionCode: sessionCode.Token,
	}, nil
}
