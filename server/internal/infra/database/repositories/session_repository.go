package repositories

import (
	"context"

	"github.com/gate-keeper/internal/domain/entities"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	"github.com/google/uuid"
	"github.com/jackc/pgx/v5/pgtype"
)

// ISessionRepository defines all operations related to the SessionCode entity.
type ISessionRepository interface {
	AddSessionCode(ctx context.Context, sessionCode *entities.SessionCode) error
	GetAuthorizationSession(ctx context.Context, userID uuid.UUID, sessionCodeToken string) (*entities.SessionCode, error)
	DeleteSessionCodeByID(ctx context.Context, sessionCodeID uuid.UUID) error
}

// SessionRepository is the shared implementation for SessionCode-related DB operations.
type SessionRepository struct {
	Store *pgstore.Queries
}

func (r SessionRepository) AddSessionCode(ctx context.Context, sessionCode *entities.SessionCode) error {
	return r.Store.AddAuthorizationSession(ctx, pgstore.AddAuthorizationSessionParams{
		ID:        sessionCode.ID,
		UserID:    sessionCode.UserID,
		Token:     sessionCode.Token,
		CreatedAt: pgtype.Timestamp{Time: sessionCode.CreatedAt, Valid: true},
		ExpiresAt: pgtype.Timestamp{Time: sessionCode.ExpiresAt, Valid: true},
		IsUsed:    sessionCode.IsUsed,
	})
}

func (r SessionRepository) GetAuthorizationSession(ctx context.Context, userID uuid.UUID, sessionCodeToken string) (*entities.SessionCode, error) {
	sessionCode, err := r.Store.GetAuthorizationSession(ctx, pgstore.GetAuthorizationSessionParams{
		Token:  sessionCodeToken,
		UserID: userID,
	})

	if err == ErrNoRows {
		return nil, nil
	}

	if err != nil {
		return nil, err
	}

	return &entities.SessionCode{
		ID:        sessionCode.ID,
		UserID:    sessionCode.UserID,
		Token:     sessionCode.Token,
		CreatedAt: sessionCode.CreatedAt.Time,
		ExpiresAt: sessionCode.ExpiresAt.Time,
		IsUsed:    sessionCode.IsUsed,
	}, nil
}

func (r SessionRepository) DeleteSessionCodeByID(ctx context.Context, sessionCodeID uuid.UUID) error {
	return r.Store.DeleteAuthorizationSession(ctx, sessionCodeID)
}
