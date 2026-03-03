package getapplicationuserbyid

import (
	"context"

	"github.com/gate-keeper/internal/domain/entities"
	"github.com/gate-keeper/internal/infra/database/repositories"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	"github.com/google/uuid"
)

type IRepository interface {
	GetOrganizationByID(ctx context.Context, organizationID uuid.UUID) (*entities.Organization, error)
	ListApplicationsFromOrganization(ctx context.Context, organizationID uuid.UUID) (*[]entities.Application, error)
	GetUserByID(ctx context.Context, userID uuid.UUID) (*entities.ApplicationUser, error)
	GetUserProfileByID(ctx context.Context, userID uuid.UUID) (*entities.UserProfile, error)
	GetRolesByUserID(ctx context.Context, userID uuid.UUID) ([]entities.ApplicationRole, error)
	GetUserMfaMethods(ctx context.Context, userID uuid.UUID) ([]*entities.MfaMethod, error)
	GetApplicationByID(ctx context.Context, applicationID uuid.UUID) (*entities.Application, error)
}

type Repository struct {
	repositories.OrganizationRepository
	repositories.ApplicationRepository
	repositories.UserRepository
	repositories.UserProfileRepository
	repositories.RoleRepository
	repositories.MfaRepository
}

func NewRepository(q *pgstore.Queries) Repository {
	return Repository{
		OrganizationRepository: repositories.OrganizationRepository{Store: q},
		ApplicationRepository:  repositories.ApplicationRepository{Store: q},
		UserRepository:         repositories.UserRepository{Store: q},
		UserProfileRepository:  repositories.UserProfileRepository{Store: q},
		RoleRepository:         repositories.RoleRepository{Store: q},
		MfaRepository:          repositories.MfaRepository{Store: q},
	}
}
