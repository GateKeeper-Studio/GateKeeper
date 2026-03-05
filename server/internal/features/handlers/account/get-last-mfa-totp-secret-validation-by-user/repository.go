package accountgetlastmfatotpsecret

import (
	"context"

	"github.com/gate-keeper/internal/domain/entities"
	"github.com/gate-keeper/internal/infra/database/repositories"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	"github.com/google/uuid"
)

type IRepository interface {
	GetUserByID(ctx context.Context, userID uuid.UUID) (*entities.TenantUser, error)
	GetLastValidMfaTotpSecretByUserID(ctx context.Context, userID uuid.UUID) (*entities.MfaUserSecret, error)
	DeleteExpiredMfaTotpSecretValidationByUserID(ctx context.Context, userID uuid.UUID) error
	GetApplicationByID(ctx context.Context, applicationID uuid.UUID) (*entities.Application, error)
}

type Repository struct {
	repositories.UserRepository
	repositories.MfaRepository
	repositories.ApplicationRepository
}

func NewRepository(q *pgstore.Queries) Repository {
	return Repository{
		UserRepository:        repositories.UserRepository{Store: q},
		MfaRepository:         repositories.MfaRepository{Store: q},
		ApplicationRepository: repositories.ApplicationRepository{Store: q},
	}
}
