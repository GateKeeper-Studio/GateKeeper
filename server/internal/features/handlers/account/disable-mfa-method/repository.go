package accountdisablemfamethod

import (
	"context"

	"github.com/gate-keeper/internal/domain/entities"
	"github.com/gate-keeper/internal/infra/database/repositories"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	"github.com/google/uuid"
)

type IRepository interface {
	GetUserByID(ctx context.Context, userID uuid.UUID) (*entities.TenantUser, error)
	UpdateUser(ctx context.Context, user *entities.TenantUser) (*entities.TenantUser, error)
	GetMfaMethodByUserID(ctx context.Context, userID uuid.UUID, method string) (*entities.MfaMethod, error)
	DisableMfaMethod(ctx context.Context, methodID uuid.UUID) error
	RevokeTotpSecretsByUserID(ctx context.Context, userID uuid.UUID) error
	DeleteBackupCodesByUserID(ctx context.Context, userID uuid.UUID) error
	GetUserMfaMethods(ctx context.Context, userID uuid.UUID) ([]*entities.MfaMethod, error)
	AddAuditLog(ctx context.Context, auditLog *entities.AuditLog) error
}

type Repository struct {
	repositories.UserRepository
	repositories.MfaRepository
	repositories.BackupCodeRepository
	repositories.AuditLogRepository
}

func NewRepository(q *pgstore.Queries) Repository {
	return Repository{
		UserRepository:       repositories.UserRepository{Store: q},
		MfaRepository:        repositories.MfaRepository{Store: q},
		BackupCodeRepository: repositories.BackupCodeRepository{Store: q},
		AuditLogRepository:   repositories.AuditLogRepository{Store: q},
	}
}
