package getprovidersdatabyapplicationid

import (
	"context"

	"github.com/gate-keeper/internal/domain/entities"
	"github.com/gate-keeper/internal/infra/database/repositories"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	"github.com/google/uuid"
)

type IRepository interface {
	GetApplicationOAuthProviders(ctx context.Context, applicationID uuid.UUID) (*[]entities.ApplicationOAuthProvider, error)
	CheckIfApplicationExists(ctx context.Context, applicationID uuid.UUID) (bool, error)
}

type Repository struct {
	repositories.OAuthProviderRepository
	repositories.ApplicationRepository
}

func NewRepository(q *pgstore.Queries) Repository {
	return Repository{
		OAuthProviderRepository: repositories.OAuthProviderRepository{Store: q},
		ApplicationRepository:   repositories.ApplicationRepository{Store: q},
	}
}
