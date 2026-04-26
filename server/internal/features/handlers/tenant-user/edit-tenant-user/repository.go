package edittenantuser

import (
	"context"

	"github.com/gate-keeper/internal/domain/entities"
	"github.com/gate-keeper/internal/infra/database/repositories"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	"github.com/google/uuid"
)

type IRepository interface {
	GetApplicationByID(ctx context.Context, applicationID uuid.UUID) (*entities.Application, error)
	GetTenantByID(ctx context.Context, id uuid.UUID) (*entities.Tenant, error)
	GetUserByID(ctx context.Context, userID uuid.UUID) (*entities.TenantUser, error)
	UpdateUser(ctx context.Context, user *entities.TenantUser) (*entities.TenantUser, error)
	EditUserProfile(ctx context.Context, updatedUser *entities.UserProfile) error
	GetRolesByUserID(ctx context.Context, userID uuid.UUID) ([]entities.ApplicationRole, error)
	RemoveUserRole(ctx context.Context, userRole *entities.UserRole) error
	AddUserRole(ctx context.Context, newUserRole *entities.UserRole) error
	ListRolesFromApplication(ctx context.Context, applicationID uuid.UUID) (*[]entities.ApplicationRole, error)
	UpdateUserCredentials(ctx context.Context, userCredentials *entities.UserCredentials) error
	GetUserCredentialsByUserID(ctx context.Context, userID uuid.UUID) (*entities.UserCredentials, error)
}

type Repository struct {
	repositories.ApplicationRepository
	repositories.TenantRepository
	repositories.UserRepository
	repositories.UserProfileRepository
	repositories.RoleRepository
	repositories.UserCredentialsRepository
}

func NewRepository(q *pgstore.Queries) Repository {
	return Repository{
		ApplicationRepository:     repositories.ApplicationRepository{Store: q},
		TenantRepository:          repositories.TenantRepository{Store: q},
		UserRepository:            repositories.UserRepository{Store: q},
		UserProfileRepository:     repositories.UserProfileRepository{Store: q},
		RoleRepository:            repositories.RoleRepository{Store: q},
		UserCredentialsRepository: repositories.UserCredentialsRepository{Store: q},
	}
}
