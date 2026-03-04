package repositories

import (
	"context"

	"github.com/gate-keeper/internal/domain/entities"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	"github.com/google/uuid"
	"github.com/jackc/pgx/v5/pgtype"
)

// IStepUpTokenRepository defines all operations related to the StepUpToken entity.
type IStepUpTokenRepository interface {
	AddStepUpToken(ctx context.Context, token *entities.StepUpToken) error
	GetStepUpTokenByToken(ctx context.Context, userID uuid.UUID, token string) (*entities.StepUpToken, error)
	MarkStepUpTokenUsed(ctx context.Context, tokenID uuid.UUID) error
	RevokeStepUpTokensByUserID(ctx context.Context, userID uuid.UUID) error
}

// StepUpTokenRepository is the shared implementation for StepUpToken-related DB operations.
type StepUpTokenRepository struct {
	Store *pgstore.Queries
}

func (r StepUpTokenRepository) AddStepUpToken(ctx context.Context, token *entities.StepUpToken) error {
	return r.Store.AddStepUpToken(ctx, pgstore.AddStepUpTokenParams{
		ID:            token.ID,
		UserID:        token.UserID,
		ApplicationID: token.ApplicationID,
		Token:         token.Token,
		CreatedAt:     pgtype.Timestamp{Time: token.CreatedAt, Valid: true},
		ExpiresAt:     pgtype.Timestamp{Time: token.ExpiresAt, Valid: true},
		IsUsed:        token.IsUsed,
	})
}

func (r StepUpTokenRepository) GetStepUpTokenByToken(ctx context.Context, userID uuid.UUID, tokenStr string) (*entities.StepUpToken, error) {
	token, err := r.Store.GetStepUpTokenByToken(ctx, pgstore.GetStepUpTokenByTokenParams{
		UserID: userID,
		Token:  tokenStr,
	})
	if err == ErrNoRows {
		return nil, nil
	}
	if err != nil {
		return nil, err
	}

	return &entities.StepUpToken{
		ID:            token.ID,
		UserID:        token.UserID,
		ApplicationID: token.ApplicationID,
		Token:         token.Token,
		CreatedAt:     token.CreatedAt.Time,
		ExpiresAt:     token.ExpiresAt.Time,
		IsUsed:        token.IsUsed,
	}, nil
}

func (r StepUpTokenRepository) MarkStepUpTokenUsed(ctx context.Context, tokenID uuid.UUID) error {
	return r.Store.MarkStepUpTokenUsed(ctx, tokenID)
}

func (r StepUpTokenRepository) RevokeStepUpTokensByUserID(ctx context.Context, userID uuid.UUID) error {
	return r.Store.RevokeStepUpTokensByUserID(ctx, userID)
}
