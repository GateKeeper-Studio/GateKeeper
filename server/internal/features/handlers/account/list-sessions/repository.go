package accountlistsessions

import (
	"context"

	"github.com/gate-keeper/internal/domain/entities"
	"github.com/gate-keeper/internal/infra/database/repositories"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	"github.com/google/uuid"
)

type IRepository interface {
	GetActiveUserSessions(ctx context.Context, userID uuid.UUID) ([]*entities.UserSession, error)
}

type Repository struct {
	repositories.UserSessionRepository
}

func NewRepository(q *pgstore.Queries) Repository {
	return Repository{
		UserSessionRepository: repositories.UserSessionRepository{Store: q},
	}
}
