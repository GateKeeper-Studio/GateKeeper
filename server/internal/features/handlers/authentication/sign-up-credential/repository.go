package signupcredential

import (
	"context"

	"github.com/gate-keeper/internal/domain/entities"
	"github.com/gate-keeper/internal/infra/database/repositories"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	"github.com/google/uuid"
)

type IRepository interface {
	IsUserExistsByEmail(ctx context.Context, email string, applicationID uuid.UUID) (bool, error)
	GetApplicationByID(ctx context.Context, applicationID uuid.UUID) (*entities.Application, error)
	GetTenantByID(ctx context.Context, id uuid.UUID) (*entities.Tenant, error)
	AddUserProfile(ctx context.Context, newUserProfile *entities.UserProfile) error
	AddEmailConfirmation(ctx context.Context, emailConfirmation *entities.EmailConfirmation) error
	AddUser(ctx context.Context, newUser *entities.TenantUser) error
	AddUserCredentials(ctx context.Context, userCredentials *entities.UserCredentials) error
}

type Repository struct {
	repositories.UserRepository
	repositories.ApplicationRepository
	repositories.TenantRepository
	repositories.UserProfileRepository
	repositories.EmailConfirmationRepository
	repositories.UserCredentialsRepository
}

func NewRepository(q *pgstore.Queries) Repository {
	return Repository{
		UserRepository:              repositories.UserRepository{Store: q},
		ApplicationRepository:       repositories.ApplicationRepository{Store: q},
		TenantRepository:            repositories.TenantRepository{Store: q},
		UserProfileRepository:       repositories.UserProfileRepository{Store: q},
		EmailConfirmationRepository: repositories.EmailConfirmationRepository{Store: q},
		UserCredentialsRepository:   repositories.UserCredentialsRepository{Store: q},
	}
}
