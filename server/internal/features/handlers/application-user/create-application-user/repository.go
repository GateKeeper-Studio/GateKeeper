package createapplicationuser

import (
	"context"
	"github.com/gate-keeper/internal/domain/entities"
	"github.com/gate-keeper/internal/infra/database/repositories"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	"github.com/google/uuid"
)

type IRepository interface {
	GetApplicationByID(ctx context.Context, applicationID uuid.UUID) (*entities.Application, error)
	IsUserExistsByEmail(ctx context.Context, email string, applicationID uuid.UUID) (bool, error)
	AddUser(ctx context.Context, user *entities.ApplicationUser) error
	AddUserProfile(ctx context.Context, userProfile *entities.UserProfile) error
	AddUserRole(ctx context.Context, userRole *entities.UserRole) error
	ListRolesFromApplication(ctx context.Context, applicationID uuid.UUID) (*[]entities.ApplicationRole, error)
	AddUserCredentials(ctx context.Context, userCredentials *entities.UserCredentials) error
}

type Repository struct {
	repositories.ApplicationRepository
	repositories.UserRepository
	repositories.UserProfileRepository
	repositories.RoleRepository
	repositories.UserCredentialsRepository
}

func NewRepository(q *pgstore.Queries) Repository {
	return Repository{
		ApplicationRepository: repositories.ApplicationRepository{Store: q},
		UserRepository: repositories.UserRepository{Store: q},
		UserProfileRepository: repositories.UserProfileRepository{Store: q},
		RoleRepository: repositories.RoleRepository{Store: q},
		UserCredentialsRepository: repositories.UserCredentialsRepository{Store: q},
	}
}
