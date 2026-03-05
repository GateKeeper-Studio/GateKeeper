package gettenantuser

import (
	"context"

	"github.com/gate-keeper/internal/domain/entities"
	"github.com/gate-keeper/internal/infra/database/repositories"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	"github.com/google/uuid"
)

type IRepository interface {
	GetTenantByID(ctx context.Context, tenantID uuid.UUID) (*entities.Tenant, error)
	GetUserByID(ctx context.Context, userID uuid.UUID) (*entities.TenantUser, error)
	GetUserProfileByID(ctx context.Context, userID uuid.UUID) (*entities.UserProfile, error)
	GetRolesByUserID(ctx context.Context, userID uuid.UUID) ([]entities.ApplicationRole, error)
	GetUserMfaMethods(ctx context.Context, userID uuid.UUID) ([]*entities.MfaMethod, error)
}

type Repository struct {
	repositories.TenantRepository
	repositories.UserRepository
	repositories.UserProfileRepository
	repositories.RoleRepository
	repositories.MfaRepository
}

func NewRepository(q *pgstore.Queries) Repository {
	return Repository{
		TenantRepository:      repositories.TenantRepository{Store: q},
		UserRepository:        repositories.UserRepository{Store: q},
		UserProfileRepository: repositories.UserProfileRepository{Store: q},
		RoleRepository:        repositories.RoleRepository{Store: q},
		MfaRepository:         repositories.MfaRepository{Store: q},
	}
}
