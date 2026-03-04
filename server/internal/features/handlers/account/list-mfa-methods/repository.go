package accountlistmfamethods

import (
	"context"

	"github.com/gate-keeper/internal/domain/entities"
	"github.com/gate-keeper/internal/infra/database/repositories"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	"github.com/google/uuid"
)

type IRepository interface {
	GetUserByID(ctx context.Context, userID uuid.UUID) (*entities.ApplicationUser, error)
	GetUserMfaMethods(ctx context.Context, userID uuid.UUID) ([]*entities.MfaMethod, error)
}

type Repository struct {
	repositories.UserRepository
	repositories.MfaRepository
}

func NewRepository(q *pgstore.Queries) Repository {
	return Repository{
		UserRepository: repositories.UserRepository{Store: q},
		MfaRepository:  repositories.MfaRepository{Store: q},
	}
}
