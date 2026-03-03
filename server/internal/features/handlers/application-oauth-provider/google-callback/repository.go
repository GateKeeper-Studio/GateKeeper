package googlecallback

import (
	"context"
	"github.com/gate-keeper/internal/domain/entities"
	"github.com/gate-keeper/internal/infra/database/repositories"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	"github.com/google/uuid"
)

type IRepository interface {
	GetApplicationOAuthProviderByID(ctx context.Context, applicationOauthProviderID uuid.UUID) (*entities.ApplicationOAuthProvider, error)
	GetExternalOAuthStateByState(ctx context.Context, state string) (*entities.ExternalOAuthState, error)
	GetUserByEmail(ctx context.Context, userEmail string, applicationID uuid.UUID) (*entities.ApplicationUser, error)
	AddUser(ctx context.Context, newUser *entities.ApplicationUser) error
	AddUserProfile(ctx context.Context, newUserProfile *entities.UserProfile) error
	AddExternalIdentity(ctx context.Context, newExternalIdentity *entities.ExternalIdentity) error
	RemoveAuthorizationCode(ctx context.Context, userID, applicationID uuid.UUID) error
	AddAuthorizationCode(ctx context.Context, authorizationCode *entities.ApplicationAuthorizationCode) error
}

type Repository struct {
	repositories.OAuthProviderRepository
	repositories.UserRepository
	repositories.UserProfileRepository
	repositories.ExternalIdentityRepository
	repositories.AuthorizationCodeRepository
}

func NewRepository(q *pgstore.Queries) Repository {
	return Repository{
		OAuthProviderRepository: repositories.OAuthProviderRepository{Store: q},
		UserRepository: repositories.UserRepository{Store: q},
		UserProfileRepository: repositories.UserProfileRepository{Store: q},
		ExternalIdentityRepository: repositories.ExternalIdentityRepository{Store: q},
		AuthorizationCodeRepository: repositories.AuthorizationCodeRepository{Store: q},
	}
}
