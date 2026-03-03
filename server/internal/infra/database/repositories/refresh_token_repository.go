package repositories

import (
	"context"

	"github.com/gate-keeper/internal/domain/entities"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	"github.com/google/uuid"
	"github.com/jackc/pgx/v5/pgtype"
)

// IRefreshTokenRepository defines all operations related to the RefreshToken entity.
type IRefreshTokenRepository interface {
	AddRefreshToken(ctx context.Context, refreshToken *entities.RefreshToken) (*entities.RefreshToken, error)
	RevokeRefreshTokenFromUser(ctx context.Context, userID uuid.UUID) error
	RevokeRefreshTokenByID(ctx context.Context, sessionID uuid.UUID) error
}

// RefreshTokenRepository is the shared implementation for RefreshToken-related DB operations.
type RefreshTokenRepository struct {
	Store *pgstore.Queries
}

func (r RefreshTokenRepository) AddRefreshToken(ctx context.Context, refreshToken *entities.RefreshToken) (*entities.RefreshToken, error) {
	err := r.Store.AddRefreshToken(ctx, pgstore.AddRefreshTokenParams{
		UserID:    refreshToken.UserID,
		ID:        refreshToken.ID,
		ExpiresAt: pgtype.Timestamp{Time: refreshToken.ExpiresAt, Valid: true},
		CreatedAt: pgtype.Timestamp{Time: refreshToken.CreatedAt, Valid: true},
	})

	if err != nil {
		return nil, err
	}

	return &entities.RefreshToken{
		ID:        refreshToken.ID,
		UserID:    refreshToken.UserID,
		ExpiresAt: refreshToken.ExpiresAt,
		CreatedAt: refreshToken.CreatedAt,
	}, nil
}

func (r RefreshTokenRepository) RevokeRefreshTokenFromUser(ctx context.Context, userID uuid.UUID) error {
	return r.Store.RevokeRefreshTokenFromUser(ctx, userID)
}

func (r RefreshTokenRepository) RevokeRefreshTokenByID(ctx context.Context, sessionID uuid.UUID) error {
	return r.Store.RevokeRefreshTokenByID(ctx, sessionID)
}
