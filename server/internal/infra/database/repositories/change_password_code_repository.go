package repositories

import (
	"context"

	"github.com/gate-keeper/internal/domain/entities"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	"github.com/google/uuid"
	"github.com/jackc/pgx/v5/pgtype"
)

// IChangePasswordCodeRepository defines all operations related to the ChangePasswordCode entity.
type IChangePasswordCodeRepository interface {
	AddChangePasswordCode(ctx context.Context, changePasswordCode *entities.ChangePasswordCode) error
	RevokeAllChangePasswordCodeByUserID(ctx context.Context, userID uuid.UUID) error
	GetChangePasswordCodeByToken(ctx context.Context, userID uuid.UUID, token string) (*entities.ChangePasswordCode, error)
}

// ChangePasswordCodeRepository is the shared implementation for ChangePasswordCode-related DB operations.
type ChangePasswordCodeRepository struct {
	Store *pgstore.Queries
}

func (r ChangePasswordCodeRepository) AddChangePasswordCode(ctx context.Context, changePasswordCode *entities.ChangePasswordCode) error {
	return r.Store.AddChangePasswordCode(ctx, pgstore.AddChangePasswordCodeParams{
		ID:        changePasswordCode.ID,
		UserID:    changePasswordCode.UserID,
		Email:     changePasswordCode.Email,
		Token:     changePasswordCode.Token,
		CreatedAt: pgtype.Timestamp{Time: changePasswordCode.CreatedAt, Valid: true},
		ExpiresAt: pgtype.Timestamp{Time: changePasswordCode.ExpiresAt, Valid: true},
	})
}

func (r ChangePasswordCodeRepository) RevokeAllChangePasswordCodeByUserID(ctx context.Context, userID uuid.UUID) error {
	return r.Store.RevokeChangePasswordCodeByUserID(ctx, userID)
}

func (r ChangePasswordCodeRepository) GetChangePasswordCodeByToken(ctx context.Context, userID uuid.UUID, changePasswordCodeToken string) (*entities.ChangePasswordCode, error) {
	changePasswordCode, err := r.Store.GetChangePasswordCodeByToken(ctx, pgstore.GetChangePasswordCodeByTokenParams{
		Token:  changePasswordCodeToken,
		UserID: userID,
	})

	if err == ErrNoRows {
		return nil, nil
	}

	if err != nil {
		return nil, err
	}

	return &entities.ChangePasswordCode{
		ID:        changePasswordCode.ID,
		UserID:    changePasswordCode.UserID,
		Email:     changePasswordCode.Email,
		Token:     changePasswordCode.Token,
		CreatedAt: changePasswordCode.CreatedAt.Time,
		ExpiresAt: changePasswordCode.ExpiresAt.Time,
	}, nil
}
