package verifyemailmfa

import (
	"context"

	"github.com/gate-keeper/internal/domain/entities"
	"github.com/gate-keeper/internal/infra/database/repositories"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	"github.com/google/uuid"
	"github.com/jackc/pgx/v5/pgtype"
)

type IRepository interface {
	AddSessionCode(ctx context.Context, sessionCode *entities.SessionCode) error
	DeleteEmailMfaCodeByID(ctx context.Context, emailMfaCodeID uuid.UUID) error
	GetMfaEmailCodeByToken(ctx context.Context, mfaMethodID uuid.UUID, token string) (*entities.MfaEmailCode, error)
	GetUserByEmail(ctx context.Context, email string, applicationID uuid.UUID) (*entities.ApplicationUser, error)
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

func (r Repository) DeleteEmailMfaCodeByID(ctx context.Context, emailMfaCodeID uuid.UUID) error {
	err := r.Store.DeleteMfaEmailCode(ctx, emailMfaCodeID)

	return err
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

func (r Repository) GetMfaEmailCodeByToken(ctx context.Context, mfaMethodID uuid.UUID, token string) (*entities.MfaEmailCode, error) {

	emailConfirmation, err := r.Store.GetMfaEmailCodeByToken(ctx, pgstore.GetMfaEmailCodeByTokenParams{
		Token:       token,
		MfaMethodID: mfaMethodID,
	})

	if err == repositories.ErrNoRows {
		return nil, nil
	}

	if err != nil {
		return nil, err
	}

	return &entities.MfaEmailCode{
		ID:          emailConfirmation.ID,
		MfaMethodID: emailConfirmation.MfaMethodID,
		Token:       emailConfirmation.Token,
		CreatedAt:   emailConfirmation.CreatedAt.Time,
		ExpiresAt:   emailConfirmation.ExpiresAt.Time,
		Verified:    emailConfirmation.Verified,
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
