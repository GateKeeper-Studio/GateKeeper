package googlelogin

import (
	"context"
	"github.com/gate-keeper/internal/domain/entities"
	"github.com/gate-keeper/internal/infra/database/repositories"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	"github.com/google/uuid"
)

type IRepository interface {
	GetApplicationOAuthProviderByID(ctx context.Context, applicationOauthProviderID uuid.UUID) (*entities.ApplicationOAuthProvider, error)
	AddExternalOAuthState(ctx context.Context, state *entities.ExternalOAuthState) error
}

type Repository struct {
	repositories.OAuthProviderRepository
}

func NewRepository(q *pgstore.Queries) Repository {
	return Repository{
		OAuthProviderRepository: repositories.OAuthProviderRepository{Store: q},
	}
}
