package githubcallback

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
	GetUserByEmail(ctx context.Context, userEmail string, applicationID uuid.UUID) (*entities.TenantUser, error)
	AddUser(ctx context.Context, newUser *entities.TenantUser) error
	UpdateUser(ctx context.Context, user *entities.TenantUser) (*entities.TenantUser, error)
	AddUserProfile(ctx context.Context, newUserProfile *entities.UserProfile) error
	AddExternalIdentity(ctx context.Context, newExternalIdentity *entities.ExternalIdentity) error
	RemoveAuthorizationCode(ctx context.Context, userID, applicationID uuid.UUID) error
	AddAuthorizationCode(ctx context.Context, authorizationCode *entities.ApplicationAuthorizationCode) error
	// Adaptive MFA support
	GetApplicationByID(ctx context.Context, applicationID uuid.UUID) (*entities.Application, error)
	GetMfaMethodByUserID(ctx context.Context, userID uuid.UUID, method string) (*entities.MfaMethod, error)
	GetMfaTotpSecretValidationByUserID(ctx context.Context, userID uuid.UUID) (*entities.MfaUserSecret, error)
	AddMfaEmailCode(ctx context.Context, emailMfaCode *entities.MfaEmailCode) error
	AddMfaTotpCode(ctx context.Context, mfaTotpCode *entities.MfaTotpCode) error
	GetWebAuthnCredentialsByMfaMethodID(ctx context.Context, mfaMethodID uuid.UUID) ([]entities.MfaWebauthnCredentials, error)
	AddMfaWebauthnSession(ctx context.Context, session *entities.MfaWebauthnSession) error
	GetUserProfileByID(ctx context.Context, userID uuid.UUID) (*entities.UserProfile, error)
}

type Repository struct {
	repositories.OAuthProviderRepository
	repositories.UserRepository
	repositories.UserProfileRepository
	repositories.ExternalIdentityRepository
	repositories.AuthorizationCodeRepository
	repositories.ApplicationRepository
	repositories.MfaRepository
}

func NewRepository(q *pgstore.Queries) Repository {
	return Repository{
		OAuthProviderRepository:     repositories.OAuthProviderRepository{Store: q},
		UserRepository:              repositories.UserRepository{Store: q},
		UserProfileRepository:       repositories.UserProfileRepository{Store: q},
		ExternalIdentityRepository:  repositories.ExternalIdentityRepository{Store: q},
		AuthorizationCodeRepository: repositories.AuthorizationCodeRepository{Store: q},
		ApplicationRepository:       repositories.ApplicationRepository{Store: q},
		MfaRepository:               repositories.MfaRepository{Store: q},
	}
}
