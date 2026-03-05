package revokeusersession

import (
	"context"

	"github.com/gate-keeper/internal/infra/database/repositories"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	"github.com/google/uuid"
)

type IRepository interface {
	RevokeRefreshTokenByID(ctx context.Context, sessionID uuid.UUID) error
}

type Repository struct {
	repositories.RefreshTokenRepository
}

func NewRepository(q *pgstore.Queries) Repository {
	return Repository{
		RefreshTokenRepository: repositories.RefreshTokenRepository{Store: q},
	}
}
