package forgotpassword

import (
	"context"
	"github.com/gate-keeper/internal/domain/entities"
	"github.com/gate-keeper/internal/infra/database/repositories"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	"github.com/google/uuid"
)

type IRepository interface {
	GetUserByID(ctx context.Context, userID uuid.UUID) (*entities.ApplicationUser, error)
	GetUserByEmail(ctx context.Context, userEmail string, applicationID uuid.UUID) (*entities.ApplicationUser, error)
	GetUserProfileByID(ctx context.Context, userID uuid.UUID) (*entities.UserProfile, error)
	CreatePasswordReset(ctx context.Context, passwordReset *entities.PasswordResetToken) error
	DeletePasswordResetFromUser(ctx context.Context, userID uuid.UUID) error
}

type Repository struct {
	repositories.UserRepository
	repositories.UserProfileRepository
	repositories.PasswordResetRepository
}

func NewRepository(q *pgstore.Queries) Repository {
	return Repository{
		UserRepository: repositories.UserRepository{Store: q},
		UserProfileRepository: repositories.UserProfileRepository{Store: q},
		PasswordResetRepository: repositories.PasswordResetRepository{Store: q},
	}
}
