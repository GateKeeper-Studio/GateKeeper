package accountgeneratebackupcodes

import (
	"context"

	"github.com/gate-keeper/internal/domain/entities"
	"github.com/gate-keeper/internal/infra/database/repositories"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	"github.com/google/uuid"
)

type IRepository interface {
	GetUserByID(ctx context.Context, userID uuid.UUID) (*entities.ApplicationUser, error)
	DeleteBackupCodesByUserID(ctx context.Context, userID uuid.UUID) error
	AddBackupCode(ctx context.Context, code *entities.BackupCode) error
	AddAuditLog(ctx context.Context, auditLog *entities.AuditLog) error
}

type Repository struct {
	repositories.UserRepository
	repositories.BackupCodeRepository
	repositories.AuditLogRepository
}

func NewRepository(q *pgstore.Queries) Repository {
	return Repository{
		UserRepository:       repositories.UserRepository{Store: q},
		BackupCodeRepository: repositories.BackupCodeRepository{Store: q},
		AuditLogRepository:   repositories.AuditLogRepository{Store: q},
	}
}
