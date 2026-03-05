package deletetenantuser

import (
	"context"

	"github.com/gate-keeper/internal/infra/database/repositories"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	"github.com/google/uuid"
)

type IRepository interface {
	DeleteTenantUser(ctx context.Context, tenantID, userID uuid.UUID) error
}

type Repository struct {
	repositories.UserRepository
}

func NewRepository(q *pgstore.Queries) Repository {
	return Repository{
		UserRepository: repositories.UserRepository{Store: q},
	}
}
