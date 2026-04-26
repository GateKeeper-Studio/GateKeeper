package getapplicationbyid

import (
	"context"

	"github.com/gate-keeper/internal/domain/entities"
	"github.com/gate-keeper/internal/infra/database/repositories"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	"github.com/google/uuid"
)

type IRepository interface {
	GetApplicationByID(ctx context.Context, applicationID uuid.UUID) (*entities.Application, error)
	ListSecretsFromApplication(ctx context.Context, applicationID uuid.UUID) (*[]entities.ApplicationSecret, error)
	GetTenantByID(ctx context.Context, id uuid.UUID) (*entities.Tenant, error)
}

type Repository struct {
	repositories.ApplicationRepository
	repositories.SecretRepository
	repositories.TenantRepository
}

func NewRepository(q *pgstore.Queries) Repository {
	return Repository{
		ApplicationRepository: repositories.ApplicationRepository{Store: q},
		SecretRepository:      repositories.SecretRepository{Store: q},
		TenantRepository:      repositories.TenantRepository{Store: q},
	}
}
