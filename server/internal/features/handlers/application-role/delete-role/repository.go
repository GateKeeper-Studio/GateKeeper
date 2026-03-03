package deleterole

import (
	"context"
	"github.com/gate-keeper/internal/infra/database/repositories"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	"github.com/google/uuid"
)

type IRepository interface {
	CheckIfApplicationExists(ctx context.Context, applicationID uuid.UUID) (bool, error)
	RemoveRole(ctx context.Context, roleID uuid.UUID) error
}

type Repository struct {
	repositories.ApplicationRepository
	repositories.RoleRepository
}

func NewRepository(q *pgstore.Queries) Repository {
	return Repository{
		ApplicationRepository: repositories.ApplicationRepository{Store: q},
		RoleRepository: repositories.RoleRepository{Store: q},
	}
}
