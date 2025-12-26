package authorize

import (
	"context"

	"github.com/gate-keeper/internal/domain/entities"
	"github.com/gate-keeper/internal/infra/database/repositories"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	"github.com/google/uuid"
	"github.com/jackc/pgx/v5/pgtype"
)

type IRepository interface {
	GetUserByEmail(ctx context.Context, email string, applicationID uuid.UUID) (*entities.ApplicationUser, error)
	GetAuthorizationSession(ctx context.Context, userID uuid.UUID, sessionCodeToken string) (*entities.SessionCode, error)
	DeleteSessionCodeByID(ctx context.Context, sessionCodeID uuid.UUID) error
	RemoveAuthorizationCode(ctx context.Context, userID, applicationID uuid.UUID) error
	AddAuthorizationCode(ctx context.Context, authorizationCode *entities.ApplicationAuthorizationCode) error
	GetMfaTotpCodeByID(ctx context.Context, id uuid.UUID) (*entities.MfaTotpCode, error)
	DeleteMfaTotpCodeByID(ctx context.Context, id uuid.UUID) error
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

func (r Repository) GetMfaTotpCodeByID(ctx context.Context, id uuid.UUID) (*entities.MfaTotpCode, error) {
	appMfaCode, err := r.Store.GetMfaTotpCodeByID(ctx, id)

	if err == repositories.ErrNoRows {
		return nil, nil
	}

	if err != nil {
		return nil, err
	}

	return &entities.MfaTotpCode{
		ID:          appMfaCode.ID,
		MfaMethodID: appMfaCode.MfaMethodID,
		Secret:      appMfaCode.Secret,
		CreatedAt:   appMfaCode.CreatedAt.Time,
	}, nil
}

func (r Repository) DeleteMfaTotpCodeByID(ctx context.Context, id uuid.UUID) error {
	err := r.Store.DeleteMfaTotpCode(ctx, id)

	if err != nil {
		return err
	}

	return nil
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
		ID:               user.ID,
		Email:            user.Email,
		CreatedAt:        user.CreatedAt.Time,
		UpdatedAt:        user.UpdatedAt,
		IsActive:         user.IsActive,
		IsEmailConfirmed: user.IsEmailConfirmed,
		ApplicationID:    user.ApplicationID,
	}, nil
}

func (r Repository) GetAuthorizationSession(ctx context.Context, userID uuid.UUID, sessionCodeToken string) (*entities.SessionCode, error) {
	emailConfirmation, err := r.Store.GetAuthorizationSession(ctx, pgstore.GetAuthorizationSessionParams{
		Token:  sessionCodeToken,
		UserID: userID,
	})

	if err == repositories.ErrNoRows {
		return nil, nil
	}

	if err != nil {
		return nil, err
	}

	return &entities.SessionCode{
		ID:        emailConfirmation.ID,
		UserID:    emailConfirmation.UserID,
		Token:     emailConfirmation.Token,
		CreatedAt: emailConfirmation.CreatedAt.Time,
		ExpiresAt: emailConfirmation.ExpiresAt.Time,
		IsUsed:    emailConfirmation.IsUsed,
	}, nil
}

func (r Repository) DeleteSessionCodeByID(ctx context.Context, sessionCodeID uuid.UUID) error {
	err := r.Store.DeleteAuthorizationSession(ctx, sessionCodeID)

	return err
}

func (r Repository) RemoveAuthorizationCode(ctx context.Context, userID, applicationID uuid.UUID) error {
	err := r.Store.RemoveAuthorizationCode(ctx, pgstore.RemoveAuthorizationCodeParams{
		ApplicationID: applicationID,
		UserID:        userID,
	})

	return err
}

func (r Repository) AddAuthorizationCode(ctx context.Context, newAuthorizationCode *entities.ApplicationAuthorizationCode) error {
	err := r.Store.AddAuthorizationCode(ctx, pgstore.AddAuthorizationCodeParams{
		ID:                  newAuthorizationCode.ID,
		ApplicationID:       newAuthorizationCode.ApplicationID,
		UserID:              newAuthorizationCode.ApplicationUserId,
		ExpiredAt:           pgtype.Timestamp{Time: newAuthorizationCode.ExpiresAt, Valid: true},
		Code:                newAuthorizationCode.Code,
		RedirectUri:         newAuthorizationCode.RedirectUri,
		CodeChallenge:       newAuthorizationCode.CodeChallenge,
		CodeChallengeMethod: newAuthorizationCode.CodeChallengeMethod,
	})

	return err
}
