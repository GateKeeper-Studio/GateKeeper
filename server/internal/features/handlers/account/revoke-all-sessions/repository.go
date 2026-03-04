package accountrevokeallsessions

import (
	"context"

	"github.com/gate-keeper/internal/domain/entities"
	"github.com/gate-keeper/internal/infra/database/repositories"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	"github.com/google/uuid"
)

type IRepository interface {
	RevokeAllUserSessions(ctx context.Context, userID uuid.UUID) error
	RevokeRefreshTokenFromUser(ctx context.Context, userID uuid.UUID) error
	AddAuditLog(ctx context.Context, auditLog *entities.AuditLog) error
}

type Repository struct {
	repositories.UserSessionRepository
	repositories.RefreshTokenRepository
	repositories.AuditLogRepository
}

func NewRepository(q *pgstore.Queries) Repository {
	return Repository{
		UserSessionRepository:  repositories.UserSessionRepository{Store: q},
		RefreshTokenRepository: repositories.RefreshTokenRepository{Store: q},
		AuditLogRepository:     repositories.AuditLogRepository{Store: q},
	}
}
