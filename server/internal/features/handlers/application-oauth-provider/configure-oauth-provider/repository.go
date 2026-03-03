package configureoauthprovider

import (
	"context"
	"github.com/gate-keeper/internal/domain/entities"
	"github.com/gate-keeper/internal/infra/database/repositories"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	"github.com/google/uuid"
)

type IRepository interface {
	AddApplicationOauthProvider(ctx context.Context, applicationOauthProvider *entities.ApplicationOAuthProvider) error
	UpdateApplicationOauthProvider(ctx context.Context, applicationOauthProvider *entities.ApplicationOAuthProvider) error
	CheckIfApplicationExists(ctx context.Context, applicationID uuid.UUID) (bool, error)
	GetApplicationOauthProviderByName(ctx context.Context, applicationID uuid.UUID, name string) (*entities.ApplicationOAuthProvider, error)
	CheckIfApplicationOauthProviderConfigurationExists(ctx context.Context, applicationID uuid.UUID, name string) (bool, error)
}

type Repository struct {
	repositories.OAuthProviderRepository
	repositories.ApplicationRepository
}

func NewRepository(q *pgstore.Queries) Repository {
	return Repository{
		OAuthProviderRepository: repositories.OAuthProviderRepository{Store: q},
		ApplicationRepository: repositories.ApplicationRepository{Store: q},
	}
}
