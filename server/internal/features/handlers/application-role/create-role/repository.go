package createrole

import (
	"context"
	"github.com/gate-keeper/internal/domain/entities"
	"github.com/gate-keeper/internal/infra/database/repositories"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	"github.com/google/uuid"
)

type IRepository interface {
	AddRole(ctx context.Context, role *entities.ApplicationRole) error
	CheckIfApplicationExists(ctx context.Context, applicationID uuid.UUID) (bool, error)
}

type Repository struct {
	repositories.RoleRepository
	repositories.ApplicationRepository
}

func NewRepository(q *pgstore.Queries) Repository {
	return Repository{
		RoleRepository: repositories.RoleRepository{Store: q},
		ApplicationRepository: repositories.ApplicationRepository{Store: q},
	}
}
