package login

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
	GetUserProfileByID(ctx context.Context, userID uuid.UUID) (*entities.UserProfile, error)
	GetMfaTotpSecretValidationByUserID(ctx context.Context, userID uuid.UUID) (*entities.MfaUserSecret, error)
	GetUserByEmail(ctx context.Context, userEmail string, applicationID uuid.UUID) (*entities.ApplicationUser, error)
	RevokeAllChangePasswordCodeByUserID(ctx context.Context, userID uuid.UUID) error
	AddMfaEmailCode(ctx context.Context, emailMfaCode *entities.MfaEmailCode) error
	AddMfaTotpCode(ctx context.Context, mfaTotpCode *entities.MfaTotpCode) error
	AddSessionCode(ctx context.Context, sessionCode *entities.SessionCode) error
	AddChangePasswordCode(ctx context.Context, changePasswordCode *entities.ChangePasswordCode) error
	GetMfaMethodByUserID(ctx context.Context, userID uuid.UUID, method string) (*entities.MfaMethod, error)
	GetUserCredentialsByUserID(ctx context.Context, userID uuid.UUID) (*entities.UserCredentials, error)
}

type Repository struct {
	Store *pgstore.Queries
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

func (r Repository) GetMfaTotpSecretValidationByUserID(ctx context.Context, userID uuid.UUID) (*entities.MfaUserSecret, error) {
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

func (r Repository) AddSessionCode(ctx context.Context, sessionCode *entities.SessionCode) error {
	err := r.Store.AddAuthorizationSession(ctx, pgstore.AddAuthorizationSessionParams{
		ID:        sessionCode.ID,
		UserID:    sessionCode.UserID,
		Token:     sessionCode.Token,
		CreatedAt: pgtype.Timestamp{Time: sessionCode.CreatedAt, Valid: true},
		ExpiresAt: pgtype.Timestamp{Time: sessionCode.ExpiresAt, Valid: true},
		IsUsed:    sessionCode.IsUsed,
	})

	return err
}

func (r Repository) AddMfaEmailCode(ctx context.Context, emailMfaCode *entities.MfaEmailCode) error {
	err := r.Store.AddMfaEmailCode(ctx, pgstore.AddMfaEmailCodeParams{
		ID:          emailMfaCode.ID,
		MfaMethodID: emailMfaCode.MfaMethodID,
		Token:       emailMfaCode.Token,
		CreatedAt:   pgtype.Timestamp{Time: emailMfaCode.CreatedAt, Valid: true},
		ExpiresAt:   pgtype.Timestamp{Time: emailMfaCode.ExpiresAt, Valid: true},
		Verified:    emailMfaCode.Verified,
	})

	return err
}

func (r Repository) AddMfaTotpCode(ctx context.Context, mfaTotpCode *entities.MfaTotpCode) error {
	err := r.Store.AddMfaTotpCode(ctx, pgstore.AddMfaTotpCodeParams{
		ID:          mfaTotpCode.ID,
		MfaMethodID: mfaTotpCode.MfaMethodID,
		Secret:      mfaTotpCode.Secret,
		CreatedAt:   pgtype.Timestamp{Time: mfaTotpCode.CreatedAt, Valid: true},
	})

	return err
}

func (r Repository) AddChangePasswordCode(ctx context.Context, changePasswordCode *entities.ChangePasswordCode) error {
	err := r.Store.AddChangePasswordCode(ctx, pgstore.AddChangePasswordCodeParams{
		ID:        changePasswordCode.ID,
		UserID:    changePasswordCode.UserID,
		Email:     changePasswordCode.Email,
		Token:     changePasswordCode.Token,
		CreatedAt: pgtype.Timestamp{Time: changePasswordCode.CreatedAt, Valid: true},
		ExpiresAt: pgtype.Timestamp{Time: changePasswordCode.ExpiresAt, Valid: true},
	})

	return err
}

func (r Repository) RevokeAllChangePasswordCodeByUserID(ctx context.Context, userID uuid.UUID) error {
	err := r.Store.RevokeChangePasswordCodeByUserID(ctx, userID)

	return err
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

func (r Repository) GetUserByEmail(ctx context.Context, email string, applicationID uuid.UUID) (*entities.ApplicationUser, error) {
	user, err := r.Store.GetUserByEmail(ctx, pgstore.GetUserByEmailParams{
		Email:         email,
		ApplicationID: applicationID,
	})

	if err != nil {
		return nil, err
	}

	return &entities.ApplicationUser{
		ID:                 user.ID,
		Email:              user.Email,
		CreatedAt:          user.CreatedAt.Time,
		UpdatedAt:          user.UpdatedAt,
		IsActive:           user.IsActive,
		IsEmailConfirmed:   user.IsEmailConfirmed,
		ApplicationID:      user.ApplicationID,
		Preferred2FAMethod: user.Preferred2faMethod,
	}, nil
}

func (r Repository) GetUserProfileByID(ctx context.Context, userID uuid.UUID) (*entities.UserProfile, error) {
	userProfile, err := r.Store.GetUserProfileByUserId(ctx, userID)

	if err != nil {
		return nil, err
	}

	return &entities.UserProfile{
		UserID:      userProfile.UserID,
		DisplayName: userProfile.DisplayName,
		FirstName:   userProfile.FirstName,
		LastName:    userProfile.LastName,
		Address:     userProfile.Address,
		PhoneNumber: userProfile.PhoneNumber,
		PhotoURL:    userProfile.PhotoUrl,
	}, nil
}
