package listapplications

import (
	"context"

	"github.com/gate-keeper/internal/domain/entities"
	"github.com/gate-keeper/internal/infra/database/repositories"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	"github.com/google/uuid"
)

type IRepository interface {
	GetTenantByID(ctx context.Context, tenantID uuid.UUID) (*entities.Tenant, error)
	ListApplicationsFromTenant(ctx context.Context, tenantID uuid.UUID) (*[]entities.Application, error)
}

type Repository struct {
	repositories.TenantRepository
	repositories.ApplicationRepository
}

func NewRepository(q *pgstore.Queries) Repository {
	return Repository{
		TenantRepository:      repositories.TenantRepository{Store: q},
		ApplicationRepository: repositories.ApplicationRepository{Store: q},
	}
}
