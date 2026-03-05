package listtenants

import (
	"context"

	"github.com/gate-keeper/internal/domain/entities"
	"github.com/gate-keeper/internal/infra/database/repositories"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
)

type IRepository interface {
	ListTenants(ctx context.Context) (*[]entities.Tenant, error)
}

type Repository struct {
	repositories.TenantRepository
}

func NewRepository(q *pgstore.Queries) Repository {
	return Repository{
		TenantRepository: repositories.TenantRepository{Store: q},
	}
}
