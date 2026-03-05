package createtenant

import (
	"context"

	"github.com/gate-keeper/internal/domain/entities"
	"github.com/gate-keeper/internal/infra/database/repositories"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
)

type IRepository interface {
	AddTenant(ctx context.Context, application *entities.Tenant) error
}

type Repository struct {
	repositories.TenantRepository
}

func NewRepository(q *pgstore.Queries) Repository {
	return Repository{
		TenantRepository: repositories.TenantRepository{Store: q},
	}
}
