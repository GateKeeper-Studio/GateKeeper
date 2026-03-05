package removetenant

import (
	"context"

	"github.com/gate-keeper/internal/infra/database/repositories"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	"github.com/google/uuid"
)

type IRepository interface {
	RemoveTenant(ctx context.Context, tenantID uuid.UUID) error
}

type Repository struct {
	repositories.TenantRepository
}

func NewRepository(q *pgstore.Queries) Repository {
	return Repository{
		TenantRepository: repositories.TenantRepository{Store: q},
	}
}
