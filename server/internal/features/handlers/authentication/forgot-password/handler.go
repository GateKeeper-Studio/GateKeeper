package forgotpassword

import (
	"context"

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

func New(q *pgstore.Queries) repositories.ServiceHandler[Command] {
	return &Handler{
		repository:  NewRepository(q),
		mailService: &mailservice.MailService{},
	}
}

func (s *Handler) Handler(ctx context.Context, command Command) error {
	user, err := s.repository.GetUserByEmail(ctx, command.Email, command.ApplicationID)

	if err != nil {
		return err
	}

	if user == nil {
		return &errors.ErrUserNotFound
	}

	if !user.IsEmailConfirmed {
		return &errors.ErrEmailNotConfirmed
	}

	s.repository.DeletePasswordResetFromUser(ctx, user.ID)

	passwordResetToken, err := entities.NewPasswordResetToken(user.ID)

	if err != nil {
		return err
	}

	if err := s.repository.CreatePasswordReset(ctx, passwordResetToken); err != nil {
		return nil
	}

	userProfile, err := s.repository.GetUserProfileByID(ctx, user.ID)

	if err != nil {
		return nil
	}

	go func() {
		if err := s.mailService.SendForgotPasswordEmail(ctx, user.Email, userProfile.FirstName, passwordResetToken.Token, passwordResetToken.ID, command.ApplicationID); err != nil {
			panic(err)
		}
	}()

	return nil
}
