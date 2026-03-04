package accountrequestemailchange

import (
	"context"

	"github.com/gate-keeper/internal/domain/entities"
	"github.com/gate-keeper/internal/infra/database/repositories"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	"github.com/google/uuid"
)

type IRepository interface {
	GetUserByID(ctx context.Context, userID uuid.UUID) (*entities.ApplicationUser, error)
	IsUserExistsByEmail(ctx context.Context, email string, applicationID uuid.UUID) (bool, error)
	AddEmailChangeRequest(ctx context.Context, req *entities.EmailChangeRequest) error
	RevokeEmailChangeRequestsByUserID(ctx context.Context, req *entities.EmailChangeRequest) error
	AddAuditLog(ctx context.Context, auditLog *entities.AuditLog) error
}

type Repository struct {
	repositories.UserRepository
	repositories.EmailChangeRequestRepository
	repositories.AuditLogRepository
}

func NewRepository(q *pgstore.Queries) Repository {
	return Repository{
		UserRepository:               repositories.UserRepository{Store: q},
		EmailChangeRequestRepository: repositories.EmailChangeRequestRepository{Store: q},
		AuditLogRepository:           repositories.AuditLogRepository{Store: q},
	}
}
