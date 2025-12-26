package verifyappmfa

import (
	"context"

	"github.com/gate-keeper/internal/domain/entities"
	"github.com/gate-keeper/internal/infra/database/repositories"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	"github.com/google/uuid"
	"github.com/jackc/pgx/v5/pgtype"
)

type IRepository interface {
	AddAuthorizationSession(ctx context.Context, sessionCode *entities.SessionCode) error
	DeleteMfaTotpCode(ctx context.Context, appMfaCodeID uuid.UUID) error
	GetMfaTotpCodeByID(ctx context.Context, id uuid.UUID) (*entities.MfaTotpCode, error)
	GetUserByEmail(ctx context.Context, email string, applicationID uuid.UUID) (*entities.ApplicationUser, error)
	GetMfaMethodByUserIDAndMethod(ctx context.Context, userID uuid.UUID, method string) (*entities.MfaMethod, error)
}

type Repository struct {
	Store *pgstore.Queries
}

func (r Repository) GetMfaMethodByUserIDAndMethod(ctx context.Context, userID uuid.UUID, method string) (*entities.MfaMethod, error) {
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

func (r Repository) DeleteMfaTotpCode(ctx context.Context, appMfaCodeID uuid.UUID) error {
	err := r.Store.DeleteMfaTotpCode(ctx, appMfaCodeID)

	return err
}

func (r Repository) AddAuthorizationSession(ctx context.Context, sessionCode *entities.SessionCode) error {
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

func (r Repository) GetMfaTotpCodeByID(ctx context.Context, id uuid.UUID) (*entities.MfaTotpCode, error) {

	emailConfirmation, err := r.Store.GetMfaTotpCodeByID(ctx, id)

	if err == repositories.ErrNoRows {
		return nil, nil
	}

	if err != nil {
		return nil, err
	}

	return &entities.MfaTotpCode{
		ID:          emailConfirmation.ID,
		MfaMethodID: emailConfirmation.MfaMethodID,
		Secret:      emailConfirmation.Secret,
		CreatedAt:   emailConfirmation.CreatedAt.Time,
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
