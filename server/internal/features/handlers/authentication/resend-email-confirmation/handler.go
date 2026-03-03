package resendemailconfirmation

import (
	"context"
	"time"

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
		return nil
	}

	if user == nil {
		return &errors.ErrUserNotFound
	}

	emailConfirmation, err := s.repository.GetEmailConfirmationByEmail(ctx, command.Email, user.ID)

	if err != nil {
		return err
	}

	if emailConfirmation != nil && emailConfirmation.CoolDown.After(time.Now().UTC()) {
		return &errors.ErrEmailConfirmationIsInCoolDown
	}

	if emailConfirmation != nil {
		s.repository.DeleteEmailConfirmation(ctx, emailConfirmation.ID)
	}

	expiresAt := time.Now().UTC().Add(20 * time.Minute) // 20 minutes
	newEmailConfirmation := entities.NewEmailConfirmation(user.ID, user.Email, expiresAt)

	if err := s.repository.AddEmailConfirmation(ctx, newEmailConfirmation); err != nil {
		return err
	}

	userProfile, err := s.repository.GetUserProfileByID(ctx, user.ID)

	if err != nil {
		return err
	}

	if err := s.mailService.SendEmailConfirmationEmail(ctx, user.Email, userProfile.FirstName, newEmailConfirmation.Token); err != nil {
		panic(err)
	}

	return nil
}
