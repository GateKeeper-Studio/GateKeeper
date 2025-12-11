package generateauthappsecret

import (
	"context"
	"strings"
	"time"

	"github.com/gate-keeper/internal/domain/entities"
	"github.com/gate-keeper/internal/infra/database/repositories"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	"github.com/google/uuid"
	"github.com/jackc/pgx/v5/pgtype"
)

type IRepository interface {
	GetApplicationByID(ctx context.Context, applicationID uuid.UUID) (*entities.Application, error)
	GetUserByID(ctx context.Context, userID uuid.UUID) (*entities.ApplicationUser, error)
	UpdateUser(ctx context.Context, user *entities.ApplicationUser) (*entities.ApplicationUser, error)
	AddMfaTotpSecretValidation(ctx context.Context, mfaUserSecret *entities.MfaUserSecret) error
	GetMfaMethodByUserID(ctx context.Context, userID uuid.UUID, method string) (*entities.MfaMethod, error)
	AddMfaMethod(ctx context.Context, mfaMethod *entities.MfaMethod) error
	RevokeTotpSecretsByUserID(ctx context.Context, userID uuid.UUID) error
}

type Repository struct {
	Store *pgstore.Queries
}

func (r Repository) RevokeTotpSecretsByUserID(ctx context.Context, userID uuid.UUID) error {
	err := r.Store.RevokeMfaTotpSecretValidationFromUser(ctx, userID)

	return err
}

func (r Repository) AddMfaMethod(ctx context.Context, mfaMethod *entities.MfaMethod) error {
	err := r.Store.AddMfaMethod(ctx, pgstore.AddMfaMethodParams{
		ID:         mfaMethod.ID,
		UserID:     mfaMethod.UserID,
		Type:       mfaMethod.Type,
		Enabled:    mfaMethod.Enabled,
		CreatedAt:  pgtype.Timestamp{Time: mfaMethod.CreatedAt, Valid: true},
		LastUsedAt: nil,
	})

	return err
}

func (r Repository) GetMfaMethodByUserID(ctx context.Context, userID uuid.UUID, method string) (*entities.MfaMethod, error) {
	mfaMethod, err := r.Store.GetMfaMethodByUserIDAndMethod(ctx, pgstore.GetMfaMethodByUserIDAndMethodParams{
		UserID: userID,
		Type:   method,
	})

	if err == repositories.ErrNoRows {
		return nil, nil
	}

	if err != nil {
		return nil, err
	}

	return &entities.MfaMethod{
		ID:         mfaMethod.ID,
		UserID:     mfaMethod.UserID,
		Type:       mfaMethod.Type,
		Enabled:    mfaMethod.Enabled,
		CreatedAt:  mfaMethod.CreatedAt.Time,
		LastUsedAt: mfaMethod.LastUsedAt,
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

func (r Repository) UpdateUser(ctx context.Context, user *entities.ApplicationUser) (*entities.ApplicationUser, error) {
	now := time.Now().UTC()

	err := r.Store.UpdateUser(ctx, pgstore.UpdateUserParams{
		ID:                 user.ID,
		Email:              user.Email,
		PasswordHash:       user.PasswordHash,
		UpdatedAt:          &now,
		IsActive:           user.IsActive,
		IsEmailConfirmed:   user.IsEmailConfirmed,
		ShouldChangePass:   user.ShouldChangePass,
		Preferred2faMethod: user.Preferred2FAMethod,
	})

	return user, err
}

func (r Repository) GetUserByID(ctx context.Context, id uuid.UUID) (*entities.ApplicationUser, error) {
	user, err := r.Store.GetUserById(ctx, id)

	if err != nil {
		return nil, err
	}

	return &entities.ApplicationUser{
		ID:                 user.ID,
		Email:              user.Email,
		PasswordHash:       user.PasswordHash,
		CreatedAt:          user.CreatedAt.Time,
		UpdatedAt:          user.UpdatedAt,
		IsActive:           user.IsActive,
		IsEmailConfirmed:   user.IsEmailConfirmed,
		ApplicationID:      user.ApplicationID,
		ShouldChangePass:   user.ShouldChangePass,
		Preferred2FAMethod: user.Preferred2faMethod,
	}, nil
}

func (r Repository) AddMfaTotpSecretValidation(ctx context.Context, mfaUserSecret *entities.MfaUserSecret) error {
	err := r.Store.AddMfaTotpSecretValidation(ctx, pgstore.AddMfaTotpSecretValidationParams{
		ID:          mfaUserSecret.ID,
		UserID:      mfaUserSecret.UserID,
		Secret:      mfaUserSecret.Secret,
		IsValidated: mfaUserSecret.IsValidated,
		CreatedAt:   pgtype.Timestamp{Time: mfaUserSecret.CreatedAt, Valid: true},
		ExpiresAt:   pgtype.Timestamp{Time: mfaUserSecret.ExpiresAt, Valid: true},
	})

	return err
}
