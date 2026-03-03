package editorganization

import (
	"context"

	"github.com/gate-keeper/internal/domain/entities"
	"github.com/gate-keeper/internal/infra/database/repositories"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	"github.com/google/uuid"
)

type IRepository interface {
	UpdateOrganization(ctx context.Context, application *entities.Organization) error
	GetOrganizationByID(ctx context.Context, id uuid.UUID) (*entities.Organization, error)
}

type Repository struct {
	repositories.OrganizationRepository
}

func NewRepository(q *pgstore.Queries) Repository {
	return Repository{
		OrganizationRepository: repositories.OrganizationRepository{Store: q},
	}
}
