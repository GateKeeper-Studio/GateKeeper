package accountconfirmemailchange

import (
	"context"

	"github.com/gate-keeper/internal/domain/entities"
	"github.com/gate-keeper/internal/infra/database/repositories"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	"github.com/google/uuid"
)

type IRepository interface {
	GetEmailChangeRequestByToken(ctx context.Context, token string) (*entities.EmailChangeRequest, error)
	ConfirmEmailChangeRequest(ctx context.Context, req *entities.EmailChangeRequest) error
	GetUserByID(ctx context.Context, userID uuid.UUID) (*entities.TenantUser, error)
	UpdateUser(ctx context.Context, user *entities.TenantUser) (*entities.TenantUser, error)
	IsUserExistsByEmail(ctx context.Context, email string, applicationID uuid.UUID) (bool, error)
	AddAuditLog(ctx context.Context, auditLog *entities.AuditLog) error
}

type Repository struct {
	repositories.EmailChangeRequestRepository
	repositories.UserRepository
	repositories.AuditLogRepository
}

func NewRepository(q *pgstore.Queries) Repository {
	return Repository{
		EmailChangeRequestRepository: repositories.EmailChangeRequestRepository{Store: q},
		UserRepository:               repositories.UserRepository{Store: q},
		AuditLogRepository:           repositories.AuditLogRepository{Store: q},
	}
}
