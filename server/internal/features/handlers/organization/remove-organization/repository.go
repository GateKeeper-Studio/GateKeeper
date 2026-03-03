package removeorganization

import (
	"context"
	"github.com/gate-keeper/internal/infra/database/repositories"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	"github.com/google/uuid"
)

type IRepository interface {
	RemoveOrganization(ctx context.Context, organizationID uuid.UUID) error
}

type Repository struct {
	repositories.OrganizationRepository
}

func NewRepository(q *pgstore.Queries) Repository {
	return Repository{
		OrganizationRepository: repositories.OrganizationRepository{Store: q},
	}
}
