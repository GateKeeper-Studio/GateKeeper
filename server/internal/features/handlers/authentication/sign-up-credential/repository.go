package signupcredential

import (
	"context"
	"strings"

	"github.com/gate-keeper/internal/domain/entities"
	"github.com/gate-keeper/internal/infra/database/repositories"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	"github.com/google/uuid"
	"github.com/jackc/pgx/v5/pgtype"
)

type IRepository interface {
	IsUserExistsByEmail(ctx context.Context, email string, applicationID uuid.UUID) (bool, error)
	GetApplicationByID(ctx context.Context, applicationID uuid.UUID) (*entities.Application, error)
	AddUserProfile(ctx context.Context, newUserProfile *entities.UserProfile) error
	AddEmailConfirmation(ctx context.Context, emailConfirmation *entities.EmailConfirmation) error
	AddUser(ctx context.Context, newUser *entities.ApplicationUser) error
	AddUserCredentials(ctx context.Context, userCredentials *entities.UserCredentials) error
}

type Repository struct {
	Store *pgstore.Queries
}

func (r Repository) AddUserCredentials(ctx context.Context, userCredentials *entities.UserCredentials) error {
	err := r.Store.AddUserCredentials(ctx, pgstore.AddUserCredentialsParams{
		ID:                userCredentials.ID,
		UserID:            userCredentials.UserID,
		PasswordAlgorithm: userCredentials.PasswordAlgorithm,
		PasswordHash:      userCredentials.PasswordHash,
		ShouldChangePass:  userCredentials.ShouldChangePass,
		CreatedAt:         pgtype.Timestamp{Time: userCredentials.CreatedAt, Valid: true},
		UpdatedAt:         userCredentials.UpdatedAt,
	})

	return err
}

func (r Repository) AddUser(ctx context.Context, newUser *entities.ApplicationUser) error {
	err := r.Store.AddUser(ctx, pgstore.AddUserParams{
		ID:               newUser.ID,
		Email:            newUser.Email,
		ApplicationID:    newUser.ApplicationID,
		CreatedAt:        pgtype.Timestamp{Time: newUser.CreatedAt, Valid: true},
		UpdatedAt:        newUser.UpdatedAt,
		IsActive:         newUser.IsActive,
		IsEmailConfirmed: newUser.IsEmailConfirmed,
	})

	return err
}

func (r Repository) IsUserExistsByEmail(ctx context.Context, email string, applicationID uuid.UUID) (bool, error) {
	_, err := r.Store.GetUserByEmail(ctx, pgstore.GetUserByEmailParams{
		Email:         email,
		ApplicationID: applicationID,
	})

	if err != nil {
		return false, nil
	}

	return true, nil
}

func (r Repository) GetApplicationByID(ctx context.Context, applicationID uuid.UUID) (*entities.Application, error) {
	application, err := r.Store.GetApplicationByID(ctx, applicationID)

	if err == repositories.ErrNoRows {
		return nil, nil
	}

	if err != nil {
		return nil, err
	}

	return &entities.Application{
		ID:                 application.ID,
		Name:               application.Name,
		Description:        application.Description,
		OrganizationID:     application.OrganizationID,
		CreatedAt:          application.CreatedAt.Time,
		IsActive:           application.IsActive,
		HasMfaAuthApp:      application.HasMfaAuthApp,
		HasMfaEmail:        application.HasMfaEmail,
		PasswordHashSecret: application.PasswordHashSecret,
		UpdatedAt:          application.UpdatedAt,
		Badges:             strings.Split(*application.Badges, ","),
		CanSelfSignUp:      application.CanSelfSignUp,
		CanSelfForgotPass:  application.CanSelfForgotPass,
	}, nil
}

func (r Repository) AddUserProfile(ctx context.Context, newUserProfile *entities.UserProfile) error {
	err := r.Store.AddUserProfile(ctx, pgstore.AddUserProfileParams{
		UserID:      newUserProfile.UserID,
		DisplayName: newUserProfile.DisplayName,
		FirstName:   newUserProfile.FirstName,
		LastName:    newUserProfile.LastName,
		Address:     newUserProfile.Address,
		PhoneNumber: newUserProfile.PhoneNumber,
		PhotoUrl:    newUserProfile.PhotoURL,
	})

	return err
}

func (r Repository) AddEmailConfirmation(ctx context.Context, emailConfirmation *entities.EmailConfirmation) error {
	err := r.Store.AddEmailConfirmation(ctx, pgstore.AddEmailConfirmationParams{
		ID:        emailConfirmation.ID,
		UserID:    emailConfirmation.UserID,
		Email:     emailConfirmation.Email,
		Token:     emailConfirmation.Token,
		CreatedAt: pgtype.Timestamp{Time: emailConfirmation.CreatedAt, Valid: true},
		CoolDown:  pgtype.Timestamp{Time: emailConfirmation.CoolDown, Valid: true},
		ExpiresAt: pgtype.Timestamp{Time: emailConfirmation.ExpiresAt, Valid: true},
		IsUsed:    emailConfirmation.IsUsed,
	})

	return err
}
