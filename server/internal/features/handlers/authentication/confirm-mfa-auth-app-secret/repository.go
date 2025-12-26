package confirmmfaauthappsecret

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
	GetApplicationByID(ctx context.Context, applicationID uuid.UUID) (*entities.Application, error)
	GetUserByID(ctx context.Context, userID uuid.UUID) (*entities.ApplicationUser, error)
	AddMfaUserSecret(ctx context.Context, mfaUserSecret *entities.MfaUserSecret) error
	// RevokeMfaUserSecret(ctx context.Context, userID uuid.UUID) error
	GetMfaTotpSecretValidationByUserId(ctx context.Context, userID uuid.UUID) (*entities.MfaUserSecret, error)
	UpdateMfaTotpSecretValidation(ctx context.Context, mfaUserSecret *entities.MfaUserSecret) error
	GetMfaMethodByUserID(ctx context.Context, userID uuid.UUID, method string) (*entities.MfaMethod, error)
}

type Repository struct {
	Store *pgstore.Queries
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

func (r Repository) UpdateMfaTotpSecretValidation(ctx context.Context, mfaUserSecret *entities.MfaUserSecret) error {
	err := r.Store.UpdateMfaTotpSecretValidation(ctx, pgstore.UpdateMfaTotpSecretValidationParams{
		ID:          mfaUserSecret.ID,
		Secret:      mfaUserSecret.Secret,
		IsValidated: mfaUserSecret.IsValidated,
		CreatedAt:   pgtype.Timestamp{Time: mfaUserSecret.CreatedAt, Valid: true},
		ExpiresAt:   pgtype.Timestamp{Time: mfaUserSecret.ExpiresAt, Valid: true},
	})

	return err
}

func (r Repository) GetMfaTotpSecretValidationByUserId(ctx context.Context, userID uuid.UUID) (*entities.MfaUserSecret, error) {
	mfaUserSecret, err := r.Store.GetMfaTotpSecretValidationByUserId(ctx, userID)

	if err == repositories.ErrNoRows {
		return nil, nil
	}

	if err != nil {
		return nil, err
	}

	return &entities.MfaUserSecret{
		ID:          mfaUserSecret.ID,
		UserID:      mfaUserSecret.UserID,
		Secret:      mfaUserSecret.Secret,
		IsValidated: mfaUserSecret.IsValidated,
		CreatedAt:   mfaUserSecret.CreatedAt.Time,
		ExpiresAt:   mfaUserSecret.ExpiresAt.Time,
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

func (r Repository) GetUserByID(ctx context.Context, id uuid.UUID) (*entities.ApplicationUser, error) {
	user, err := r.Store.GetUserById(ctx, id)

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

func (r Repository) AddMfaUserSecret(ctx context.Context, mfaUserSecret *entities.MfaUserSecret) error {
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

// func (r Repository) RevokeMfaUserSecret(ctx context.Context, userID uuid.UUID) error {
// 	err := r.Store.RevokeMfaUserSecretFromUser(ctx, userID)

// 	if err != nil {
// 		return err
// 	}

// 	return nil
// }
