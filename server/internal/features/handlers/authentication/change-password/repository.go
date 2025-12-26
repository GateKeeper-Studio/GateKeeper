package changepassword

import (
	"context"
	"strings"
	"time"

	"github.com/gate-keeper/internal/domain/entities"
	"github.com/gate-keeper/internal/infra/database/repositories"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	"github.com/google/uuid"
)

type IRepository interface {
	GetChangePasswordCodeByToken(ctx context.Context, userID uuid.UUID, changePasswordCode string) (*entities.ChangePasswordCode, error)
	GetApplicationByID(ctx context.Context, applicationID uuid.UUID) (*entities.Application, error)
	GetUserByID(ctx context.Context, userID uuid.UUID) (*entities.ApplicationUser, error)
	UpdateUser(ctx context.Context, user *entities.ApplicationUser) (*entities.ApplicationUser, error)
	RevokeRefreshTokenFromUser(ctx context.Context, userID uuid.UUID) error
	RevokeAllChangePasswordCodeByUserID(ctx context.Context, userID uuid.UUID) error
	GetUserCredentialsByUserID(ctx context.Context, userID uuid.UUID) (*entities.UserCredentials, error)
	UpdateUserCredentials(ctx context.Context, userCredentials *entities.UserCredentials) error
}

type Repository struct {
	Store *pgstore.Queries
}

func (r Repository) UpdateUserCredentials(ctx context.Context, userCredentials *entities.UserCredentials) error {
	err := r.Store.UpdateUserCredentials(ctx, pgstore.UpdateUserCredentialsParams{
		UserID:            userCredentials.UserID,
		PasswordHash:      userCredentials.PasswordHash,
		PasswordAlgorithm: userCredentials.PasswordAlgorithm,
		ShouldChangePass:  userCredentials.ShouldChangePass,
		UpdatedAt:         userCredentials.UpdatedAt,
	})

	if err != nil {
		return err
	}

	return nil
}

func (r Repository) GetUserCredentialsByUserID(ctx context.Context, userID uuid.UUID) (*entities.UserCredentials, error) {
	userCredentials, err := r.Store.GetUserCredentialsByUserID(ctx, userID)

	if err == repositories.ErrNoRows {
		return nil, nil
	}

	if err != nil {
		return nil, err
	}

	return &entities.UserCredentials{
		ID:                userCredentials.ID,
		UserID:            userCredentials.UserID,
		PasswordAlgorithm: userCredentials.PasswordAlgorithm,
		PasswordHash:      userCredentials.PasswordHash,
		ShouldChangePass:  userCredentials.ShouldChangePass,
		CreatedAt:         userCredentials.CreatedAt.Time,
		UpdatedAt:         userCredentials.UpdatedAt,
	}, nil
}

func (r Repository) GetChangePasswordCodeByToken(ctx context.Context, userID uuid.UUID, changePasswordCodeToken string) (*entities.ChangePasswordCode, error) {
	emailConfirmation, err := r.Store.GetChangePasswordCodeByToken(ctx, pgstore.GetChangePasswordCodeByTokenParams{
		Token:  changePasswordCodeToken,
		UserID: userID,
	})

	if err == repositories.ErrNoRows {
		return nil, nil
	}

	if err != nil {
		return nil, err
	}

	return &entities.ChangePasswordCode{
		ID:        emailConfirmation.ID,
		UserID:    emailConfirmation.UserID,
		Email:     emailConfirmation.Email,
		Token:     emailConfirmation.Token,
		CreatedAt: emailConfirmation.CreatedAt.Time,
		ExpiresAt: emailConfirmation.ExpiresAt.Time,
	}, nil
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

func (r Repository) GetUserByID(ctx context.Context, userID uuid.UUID) (*entities.ApplicationUser, error) {
	user, err := r.Store.GetUserById(ctx, userID)

	if err != nil {
		return nil, err
	}

	return &entities.ApplicationUser{
		ID:               user.ID,
		Email:            user.Email,
		CreatedAt:        user.CreatedAt.Time,
		UpdatedAt:        user.UpdatedAt,
		IsActive:         user.IsActive,
		IsEmailConfirmed: user.IsEmailConfirmed,
		ApplicationID:    user.ApplicationID,
	}, nil
}

func (r Repository) UpdateUser(ctx context.Context, user *entities.ApplicationUser) (*entities.ApplicationUser, error) {
	now := time.Now().UTC()

	err := r.Store.UpdateUser(ctx, pgstore.UpdateUserParams{
		ID:               user.ID,
		Email:            user.Email,
		UpdatedAt:        &now,
		IsActive:         user.IsActive,
		IsEmailConfirmed: user.IsEmailConfirmed,
	})

	return user, err
}

func (r Repository) RevokeRefreshTokenFromUser(ctx context.Context, userID uuid.UUID) error {
	err := r.Store.RevokeRefreshTokenFromUser(ctx, userID)

	if err != nil {
		return err
	}

	return nil
}

func (r Repository) RevokeAllChangePasswordCodeByUserID(ctx context.Context, userID uuid.UUID) error {
	err := r.Store.RevokeChangePasswordCodeByUserID(ctx, userID)

	return err
}
