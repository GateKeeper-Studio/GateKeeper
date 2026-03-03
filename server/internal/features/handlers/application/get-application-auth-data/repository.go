package getapplicationauthdata

import (
	"context"
	"github.com/gate-keeper/internal/domain/entities"
	"github.com/gate-keeper/internal/infra/database/repositories"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	"github.com/google/uuid"
)

type IRepository interface {
	GetApplicationByID(ctx context.Context, applicationID uuid.UUID) (*entities.Application, error)
	GetApplicationOAuthProviders(ctx context.Context, applicationID uuid.UUID) (*[]entities.ApplicationOAuthProvider, error)
}

type Repository struct {
	repositories.ApplicationRepository
	repositories.OAuthProviderRepository
}

func NewRepository(q *pgstore.Queries) Repository {
	return Repository{
		ApplicationRepository: repositories.ApplicationRepository{Store: q},
		OAuthProviderRepository: repositories.OAuthProviderRepository{Store: q},
	}
}
