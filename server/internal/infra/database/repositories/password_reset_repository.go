package repositories

import (
	"context"

	"github.com/gate-keeper/internal/domain/entities"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	"github.com/google/uuid"
	"github.com/jackc/pgx/v5/pgtype"
)

// IPasswordResetRepository defines all operations related to the PasswordResetToken entity.
type IPasswordResetRepository interface {
	CreatePasswordReset(ctx context.Context, passwordResetToken *entities.PasswordResetToken) error
	GetPasswordResetByTokenID(ctx context.Context, tokenID uuid.UUID) (*entities.PasswordResetToken, error)
	DeletePasswordResetFromUser(ctx context.Context, userID uuid.UUID) error
}

// PasswordResetRepository is the shared implementation for PasswordResetToken-related DB operations.
type PasswordResetRepository struct {
	Store *pgstore.Queries
}

func (r PasswordResetRepository) CreatePasswordReset(ctx context.Context, passwordResetToken *entities.PasswordResetToken) error {
	return r.Store.CreatePasswordReset(ctx, pgstore.CreatePasswordResetParams{
		ID:        passwordResetToken.ID,
		UserID:    passwordResetToken.UserID,
		Token:     passwordResetToken.Token,
		CreatedAt: pgtype.Timestamp{Time: passwordResetToken.CreatedAt, Valid: true},
		ExpiresAt: pgtype.Timestamp{Time: passwordResetToken.ExpiresAt, Valid: true},
	})
}

func (r PasswordResetRepository) GetPasswordResetByTokenID(ctx context.Context, tokenID uuid.UUID) (*entities.PasswordResetToken, error) {
	passwordReset, err := r.Store.GetPasswordResetByTokenID(ctx, tokenID)

	if err == ErrNoRows {
		return nil, nil
	}

	if err != nil {
		return nil, err
	}

	return &entities.PasswordResetToken{
		ID:        passwordReset.ID,
		UserID:    passwordReset.UserID,
		Token:     passwordReset.Token,
		CreatedAt: passwordReset.CreatedAt.Time,
		ExpiresAt: passwordReset.ExpiresAt.Time,
	}, nil
}

func (r PasswordResetRepository) DeletePasswordResetFromUser(ctx context.Context, userID uuid.UUID) error {
	return r.Store.DeletePasswordResetFromUser(ctx, userID)
}
