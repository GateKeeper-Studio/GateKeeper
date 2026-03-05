package signincredential

import (
	"context"
	"github.com/gate-keeper/internal/domain/entities"
	"github.com/gate-keeper/internal/infra/database/repositories"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	"github.com/google/uuid"
)

type IRepository interface {
	GetUserByID(ctx context.Context, userID uuid.UUID) (*entities.TenantUser, error)
	GetUserProfileByID(ctx context.Context, userID uuid.UUID) (*entities.UserProfile, error)
	ListSecretsFromApplication(ctx context.Context, applicationID uuid.UUID) (*[]entities.ApplicationSecret, error)
	RemoveAuthorizationCode(ctx context.Context, userID, applicationId uuid.UUID) error
	GetAuthorizationCodeById(ctx context.Context, code uuid.UUID) (*entities.ApplicationAuthorizationCode, error)
	RevokeRefreshTokenFromUser(ctx context.Context, userID uuid.UUID) error
	AddRefreshToken(ctx context.Context, refreshToken *entities.RefreshToken) (*entities.RefreshToken, error)
	GetApplicationByID(ctx context.Context, applicationID uuid.UUID) (*entities.Application, error)
}

type Repository struct {
	repositories.UserRepository
	repositories.UserProfileRepository
	repositories.SecretRepository
	repositories.AuthorizationCodeRepository
	repositories.RefreshTokenRepository
	repositories.ApplicationRepository
}

func NewRepository(q *pgstore.Queries) Repository {
	return Repository{
		UserRepository: repositories.UserRepository{Store: q},
		UserProfileRepository: repositories.UserProfileRepository{Store: q},
		SecretRepository: repositories.SecretRepository{Store: q},
		AuthorizationCodeRepository: repositories.AuthorizationCodeRepository{Store: q},
		RefreshTokenRepository: repositories.RefreshTokenRepository{Store: q},
		ApplicationRepository: repositories.ApplicationRepository{Store: q},
	}
}
