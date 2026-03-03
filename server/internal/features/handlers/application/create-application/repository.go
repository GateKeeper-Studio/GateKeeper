package createapplication

import (
	"context"

	"github.com/gate-keeper/internal/domain/entities"
	"github.com/gate-keeper/internal/infra/database/repositories"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
)

type IRepository interface {
	AddApplication(ctx context.Context, application *entities.Application) error
	AddRole(ctx context.Context, role *entities.ApplicationRole) error
}

type Repository struct {
	repositories.ApplicationRepository
	repositories.RoleRepository
}

func NewRepository(q *pgstore.Queries) Repository {
	return Repository{
		ApplicationRepository: repositories.ApplicationRepository{Store: q},
		RoleRepository:        repositories.RoleRepository{Store: q},
	}
}
