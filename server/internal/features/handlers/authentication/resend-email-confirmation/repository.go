package resendemailconfirmation

import (
	"context"
	"github.com/gate-keeper/internal/domain/entities"
	"github.com/gate-keeper/internal/infra/database/repositories"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	"github.com/google/uuid"
)

type IRepository interface {
	GetUserProfileByID(ctx context.Context, userID uuid.UUID) (*entities.UserProfile, error)
	GetUserByEmail(ctx context.Context, userEmail string, applicationID uuid.UUID) (*entities.TenantUser, error)
	AddEmailConfirmation(ctx context.Context, emailConfirmation *entities.EmailConfirmation) error
	DeleteEmailConfirmation(ctx context.Context, emailConfirmationID uuid.UUID) error
	GetEmailConfirmationByEmail(ctx context.Context, email string, userID uuid.UUID) (*entities.EmailConfirmation, error)
}

type Repository struct {
	repositories.UserProfileRepository
	repositories.UserRepository
	repositories.EmailConfirmationRepository
}

func NewRepository(q *pgstore.Queries) Repository {
	return Repository{
		UserProfileRepository: repositories.UserProfileRepository{Store: q},
		UserRepository: repositories.UserRepository{Store: q},
		EmailConfirmationRepository: repositories.EmailConfirmationRepository{Store: q},
	}
}
