package changepassword

import (
	"context"
	"github.com/gate-keeper/internal/domain/entities"
	"github.com/gate-keeper/internal/infra/database/repositories"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	"github.com/google/uuid"
)

type IRepository interface {
	GetChangePasswordCodeByToken(ctx context.Context, userID uuid.UUID, changePasswordCode string) (*entities.ChangePasswordCode, error)
	GetApplicationByID(ctx context.Context, applicationID uuid.UUID) (*entities.Application, error)
	GetUserByID(ctx context.Context, userID uuid.UUID) (*entities.ApplicationUser, error)
	UpdateUser(ctx context.Context, user *entities.ApplicationUser) (*entities.ApplicationUser, error)
	RevokeRefreshTokenFromUser(ctx context.Context, userID uuid.UUID) error
	RevokeAllChangePasswordCodeByUserID(ctx context.Context, userID uuid.UUID) error
	GetUserCredentialsByUserID(ctx context.Context, userID uuid.UUID) (*entities.UserCredentials, error)
	UpdateUserCredentials(ctx context.Context, userCredentials *entities.UserCredentials) error
}

type Repository struct {
	repositories.ChangePasswordCodeRepository
	repositories.ApplicationRepository
	repositories.UserRepository
	repositories.RefreshTokenRepository
	repositories.UserCredentialsRepository
}

func NewRepository(q *pgstore.Queries) Repository {
	return Repository{
		ChangePasswordCodeRepository: repositories.ChangePasswordCodeRepository{Store: q},
		ApplicationRepository: repositories.ApplicationRepository{Store: q},
		UserRepository: repositories.UserRepository{Store: q},
		RefreshTokenRepository: repositories.RefreshTokenRepository{Store: q},
		UserCredentialsRepository: repositories.UserCredentialsRepository{Store: q},
	}
}
