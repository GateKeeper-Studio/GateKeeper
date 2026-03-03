package resetpassword

import (
	"context"
	"github.com/gate-keeper/internal/domain/entities"
	"github.com/gate-keeper/internal/infra/database/repositories"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	"github.com/google/uuid"
)

type IRepository interface {
	RevokeRefreshTokenFromUser(ctx context.Context, userID uuid.UUID) error
	GetPasswordResetByTokenID(ctx context.Context, tokenID uuid.UUID) (*entities.PasswordResetToken, error)
	UpdateUser(ctx context.Context, user *entities.ApplicationUser) (*entities.ApplicationUser, error)
	GetApplicationByID(ctx context.Context, applicationID uuid.UUID) (*entities.Application, error)
	GetUserByID(ctx context.Context, userID uuid.UUID) (*entities.ApplicationUser, error)
	DeletePasswordResetFromUser(ctx context.Context, userID uuid.UUID) error
	GetUserCredentialsByUserID(ctx context.Context, userID uuid.UUID) (*entities.UserCredentials, error)
	UpdateUserCredentials(ctx context.Context, userCredentials *entities.UserCredentials) error
}

type Repository struct {
	repositories.RefreshTokenRepository
	repositories.PasswordResetRepository
	repositories.UserRepository
	repositories.ApplicationRepository
	repositories.UserCredentialsRepository
}

func NewRepository(q *pgstore.Queries) Repository {
	return Repository{
		RefreshTokenRepository: repositories.RefreshTokenRepository{Store: q},
		PasswordResetRepository: repositories.PasswordResetRepository{Store: q},
		UserRepository: repositories.UserRepository{Store: q},
		ApplicationRepository: repositories.ApplicationRepository{Store: q},
		UserCredentialsRepository: repositories.UserCredentialsRepository{Store: q},
	}
}
