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
}

type Repository struct {
	repositories.ApplicationRepository
	repositories.SecretRepository
}

func NewRepository(q *pgstore.Queries) Repository {
	return Repository{
		ApplicationRepository: repositories.ApplicationRepository{Store: q},
		SecretRepository:      repositories.SecretRepository{Store: q},
	}
}
