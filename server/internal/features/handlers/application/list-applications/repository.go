package listapplications

import (
	"context"
	"github.com/gate-keeper/internal/domain/entities"
	"github.com/gate-keeper/internal/infra/database/repositories"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	"github.com/google/uuid"
)

type IRepository interface {
	GetOrganizationByID(ctx context.Context, organizationID uuid.UUID) (*entities.Organization, error)
	ListApplicationsFromOrganization(ctx context.Context, organizationID uuid.UUID) (*[]entities.Application, error)
}

type Repository struct {
	repositories.OrganizationRepository
	repositories.ApplicationRepository
}

func NewRepository(q *pgstore.Queries) Repository {
	return Repository{
		OrganizationRepository: repositories.OrganizationRepository{Store: q},
		ApplicationRepository: repositories.ApplicationRepository{Store: q},
	}
}
