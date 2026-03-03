package deletesecret

import (
	"context"
	"github.com/gate-keeper/internal/infra/database/repositories"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	"github.com/google/uuid"
)

type IRepository interface {
	CheckIfApplicationExists(ctx context.Context, applicationID uuid.UUID) (bool, error)
	RemoveSecret(ctx context.Context, secretID uuid.UUID) error
}

type Repository struct {
	repositories.ApplicationRepository
	repositories.SecretRepository
}

func NewRepository(q *pgstore.Queries) Repository {
	return Repository{
		ApplicationRepository: repositories.ApplicationRepository{Store: q},
		SecretRepository: repositories.SecretRepository{Store: q},
	}
}
