package repositories

import (
	"context"

	"github.com/gate-keeper/internal/domain/entities"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	"github.com/google/uuid"
	"github.com/jackc/pgx/v5/pgtype"
)

// IUserSessionRepository defines all operations related to UserSession entities.
type IUserSessionRepository interface {
	AddUserSession(ctx context.Context, session *entities.UserSession) error
	GetActiveUserSessions(ctx context.Context, userID uuid.UUID) ([]*entities.UserSession, error)
	GetUserSessionByID(ctx context.Context, sessionID, userID uuid.UUID) (*entities.UserSession, error)
	RevokeUserSessionByID(ctx context.Context, sessionID, userID uuid.UUID) error
	RevokeAllUserSessions(ctx context.Context, userID uuid.UUID) error
	UpdateUserSessionLastActive(ctx context.Context, sessionID uuid.UUID) error
}

// UserSessionRepository is the shared implementation for UserSession-related DB operations.
type UserSessionRepository struct {
	Store *pgstore.Queries
}

func (r UserSessionRepository) AddUserSession(ctx context.Context, session *entities.UserSession) error {
	return r.Store.AddUserSession(ctx, pgstore.AddUserSessionParams{
		ID:            session.ID,
		UserID:        session.UserID,
		ApplicationID: session.ApplicationID,
		IpAddress:     session.IPAddress,
		UserAgent:     session.UserAgent,
		Location:      session.Location,
		CreatedAt:     pgtype.Timestamp{Time: session.CreatedAt, Valid: true},
		LastActiveAt:  pgtype.Timestamp{Time: session.LastActiveAt, Valid: true},
		ExpiresAt:     pgtype.Timestamp{Time: session.ExpiresAt, Valid: true},
		IsRevoked:     session.IsRevoked,
	})
}

func (r UserSessionRepository) GetActiveUserSessions(ctx context.Context, userID uuid.UUID) ([]*entities.UserSession, error) {
	rows, err := r.Store.GetActiveUserSessions(ctx, userID)
	if err != nil {
		return nil, err
	}

	sessions := make([]*entities.UserSession, 0, len(rows))
	for _, row := range rows {
		sessions = append(sessions, &entities.UserSession{
			ID:            row.ID,
			UserID:        row.UserID,
			ApplicationID: row.ApplicationID,
			IPAddress:     row.IpAddress,
			UserAgent:     row.UserAgent,
			Location:      row.Location,
			CreatedAt:     row.CreatedAt.Time,
			LastActiveAt:  row.LastActiveAt.Time,
			ExpiresAt:     row.ExpiresAt.Time,
			IsRevoked:     row.IsRevoked,
		})
	}

	return sessions, nil
}

func (r UserSessionRepository) GetUserSessionByID(ctx context.Context, sessionID, userID uuid.UUID) (*entities.UserSession, error) {
	row, err := r.Store.GetUserSessionByID(ctx, pgstore.GetUserSessionByIDParams{
		ID:     sessionID,
		UserID: userID,
	})
	if err == ErrNoRows {
		return nil, nil
	}
	if err != nil {
		return nil, err
	}

	return &entities.UserSession{
		ID:            row.ID,
		UserID:        row.UserID,
		ApplicationID: row.ApplicationID,
		IPAddress:     row.IpAddress,
		UserAgent:     row.UserAgent,
		Location:      row.Location,
		CreatedAt:     row.CreatedAt.Time,
		LastActiveAt:  row.LastActiveAt.Time,
		ExpiresAt:     row.ExpiresAt.Time,
		IsRevoked:     row.IsRevoked,
	}, nil
}

func (r UserSessionRepository) RevokeUserSessionByID(ctx context.Context, sessionID, userID uuid.UUID) error {
	return r.Store.RevokeUserSessionByID(ctx, pgstore.RevokeUserSessionByIDParams{
		ID:     sessionID,
		UserID: userID,
	})
}

func (r UserSessionRepository) RevokeAllUserSessions(ctx context.Context, userID uuid.UUID) error {
	return r.Store.RevokeAllUserSessions(ctx, userID)
}

func (r UserSessionRepository) UpdateUserSessionLastActive(ctx context.Context, sessionID uuid.UUID) error {
	return r.Store.UpdateUserSessionLastActive(ctx, sessionID)
}
