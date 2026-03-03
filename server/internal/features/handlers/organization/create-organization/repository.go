package createorganization

import (
	"context"
	"github.com/gate-keeper/internal/domain/entities"
	"github.com/gate-keeper/internal/infra/database/repositories"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
)

type IRepository interface {
	AddOrganization(ctx context.Context, application *entities.Organization) error
}

type Repository struct {
	repositories.OrganizationRepository
}

func NewRepository(q *pgstore.Queries) Repository {
	return Repository{
		OrganizationRepository: repositories.OrganizationRepository{Store: q},
	}
}
