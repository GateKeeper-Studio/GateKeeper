package generateauthappsecret

import (
	"context"
	"github.com/gate-keeper/internal/domain/entities"
	"github.com/gate-keeper/internal/infra/database/repositories"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	"github.com/google/uuid"
)

type IRepository interface {
	GetApplicationByID(ctx context.Context, applicationID uuid.UUID) (*entities.Application, error)
	GetUserByID(ctx context.Context, userID uuid.UUID) (*entities.ApplicationUser, error)
	UpdateUser(ctx context.Context, user *entities.ApplicationUser) (*entities.ApplicationUser, error)
	AddMfaTotpSecretValidation(ctx context.Context, mfaUserSecret *entities.MfaUserSecret) error
	GetMfaMethodByUserID(ctx context.Context, userID uuid.UUID, method string) (*entities.MfaMethod, error)
	AddMfaMethod(ctx context.Context, mfaMethod *entities.MfaMethod) error
	RevokeTotpSecretsByUserID(ctx context.Context, userID uuid.UUID) error
}

type Repository struct {
	repositories.ApplicationRepository
	repositories.UserRepository
	repositories.MfaRepository
}

func NewRepository(q *pgstore.Queries) Repository {
	return Repository{
		ApplicationRepository: repositories.ApplicationRepository{Store: q},
		UserRepository: repositories.UserRepository{Store: q},
		MfaRepository: repositories.MfaRepository{Store: q},
	}
}
