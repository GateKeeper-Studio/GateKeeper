package signupcredential

import (
	"context"
	"time"

	"github.com/gate-keeper/internal/domain/entities"
	"github.com/gate-keeper/internal/domain/errors"
	application_utils "github.com/gate-keeper/internal/features/utils"
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
		repository:  Repository{Store: q},
		mailService: &mailservice.MailService{},
	}
}

func (s *Handler) Handler(ctx context.Context, command Command) error {
	isEmailValid := application_utils.EmailValidator(command.Email)

	if !isEmailValid {
		return &errors.ErrInvalidEmail
	}

	isUserExist, err := s.repository.IsUserExistsByEmail(ctx, command.Email, command.ApplicationID)

	if err != nil {
		return err
	}

	if isUserExist {
		return &errors.ErrUserAlreadyExists
	}

	application, err := s.repository.GetApplicationByID(ctx, command.ApplicationID)

	if err != nil {
		return err
	}

	hashedPassword, err := application_utils.HashPassword(command.Password, application.PasswordHashSecret)

	if err != nil {
		return err
	}

	user, err := entities.CreateApplicationUser(command.Email, &hashedPassword, command.ApplicationID, false)

	if err != nil {
		return err
	}

	userProfile := entities.NewUserProfile(
		user.ID,
		command.FirstName,
		command.LastName,
		command.DisplayName,
		nil,
		nil,
		nil,
	)

	userCredentials := entities.NewUserCredentials(
		user.ID,
		hashedPassword,
		false,
	)

	if err := s.repository.AddUser(ctx, user); err != nil {
		return err
	}

	if err := s.repository.AddUserProfile(ctx, userProfile); err != nil {
		return err
	}

	if err := s.repository.AddUserCredentials(ctx, userCredentials); err != nil {
		return err
	}

	expiresAt := time.Now().UTC().Add(20 * time.Minute)
	emailConfirmation := entities.NewEmailConfirmation(user.ID, user.Email, expiresAt)

	if err := s.repository.AddEmailConfirmation(ctx, emailConfirmation); err != nil {
		return err
	}

	go func() {
		if err := s.mailService.SendEmailConfirmationEmail(ctx, user.Email, userProfile.FirstName, emailConfirmation.Token); err != nil {
			panic(err)
		}
	}()

	return nil
}
