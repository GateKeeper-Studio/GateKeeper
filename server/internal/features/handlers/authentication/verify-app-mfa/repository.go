package verifyappmfa

import (
	"context"

	"github.com/gate-keeper/internal/domain/entities"
	"github.com/gate-keeper/internal/infra/database/repositories"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	"github.com/google/uuid"
)

type IRepository interface {
	AddSessionCode(ctx context.Context, sessionCode *entities.SessionCode) error
	DeleteMfaTotpCode(ctx context.Context, appMfaCodeID uuid.UUID) error
	GetMfaTotpCodeByID(ctx context.Context, id uuid.UUID) (*entities.MfaTotpCode, error)
	GetUserByEmail(ctx context.Context, email string, applicationID uuid.UUID) (*entities.TenantUser, error)
	GetMfaMethodByUserID(ctx context.Context, userID uuid.UUID, method string) (*entities.MfaMethod, error)
}

type Repository struct {
	repositories.SessionRepository
	repositories.MfaRepository
	repositories.UserRepository
}

func NewRepository(q *pgstore.Queries) Repository {
	return Repository{
		SessionRepository: repositories.SessionRepository{Store: q},
		MfaRepository:     repositories.MfaRepository{Store: q},
		UserRepository:    repositories.UserRepository{Store: q},
	}
}
