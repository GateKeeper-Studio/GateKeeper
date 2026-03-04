package accountrevokesession

import (
	"context"

	"github.com/gate-keeper/internal/domain/entities"
	"github.com/gate-keeper/internal/infra/database/repositories"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	"github.com/google/uuid"
)

type IRepository interface {
	GetUserSessionByID(ctx context.Context, sessionID, userID uuid.UUID) (*entities.UserSession, error)
	RevokeUserSessionByID(ctx context.Context, sessionID, userID uuid.UUID) error
	AddAuditLog(ctx context.Context, auditLog *entities.AuditLog) error
}

type Repository struct {
	repositories.UserSessionRepository
	repositories.AuditLogRepository
}

func NewRepository(q *pgstore.Queries) Repository {
	return Repository{
		UserSessionRepository: repositories.UserSessionRepository{Store: q},
		AuditLogRepository:    repositories.AuditLogRepository{Store: q},
	}
}
