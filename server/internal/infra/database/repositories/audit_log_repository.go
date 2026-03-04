package repositories

import (
	"context"

	"github.com/gate-keeper/internal/domain/entities"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	"github.com/google/uuid"
	"github.com/jackc/pgx/v5/pgtype"
)

// IAuditLogRepository defines all operations related to the AuditLog entity.
type IAuditLogRepository interface {
	AddAuditLog(ctx context.Context, auditLog *entities.AuditLog) error
	GetAuditLogsByUserID(ctx context.Context, userID uuid.UUID, limit, offset int32) ([]*entities.AuditLog, error)
}

// AuditLogRepository is the shared implementation for AuditLog-related DB operations.
type AuditLogRepository struct {
	Store *pgstore.Queries
}

func (r AuditLogRepository) AddAuditLog(ctx context.Context, auditLog *entities.AuditLog) error {
	return r.Store.AddAuditLog(ctx, pgstore.AddAuditLogParams{
		ID:            auditLog.ID,
		UserID:        auditLog.UserID,
		ApplicationID: auditLog.ApplicationID,
		EventType:     auditLog.EventType,
		IpAddress:     auditLog.IPAddress,
		UserAgent:     auditLog.UserAgent,
		Result:        auditLog.Result,
		Details:       auditLog.Details,
		CreatedAt:     pgtype.Timestamp{Time: auditLog.CreatedAt, Valid: true},
	})
}

func (r AuditLogRepository) GetAuditLogsByUserID(ctx context.Context, userID uuid.UUID, limit, offset int32) ([]*entities.AuditLog, error) {
	rows, err := r.Store.GetAuditLogsByUserID(ctx, pgstore.GetAuditLogsByUserIDParams{
		UserID:      userID,
		LimitCount:  limit,
		OffsetCount: offset,
	})
	if err != nil {
		return nil, err
	}

	logs := make([]*entities.AuditLog, 0, len(rows))
	for _, row := range rows {
		logs = append(logs, &entities.AuditLog{
			ID:            row.ID,
			UserID:        row.UserID,
			ApplicationID: row.ApplicationID,
			EventType:     row.EventType,
			IPAddress:     row.IpAddress,
			UserAgent:     row.UserAgent,
			Result:        row.Result,
			Details:       row.Details,
			CreatedAt:     row.CreatedAt.Time,
		})
	}

	return logs, nil
}
