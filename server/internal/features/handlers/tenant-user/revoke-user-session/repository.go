package revokeusersession

import (
	"context"
	"github.com/gate-keeper/internal/infra/database/repositories"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	"github.com/google/uuid"
)

type IRepository interface {
	CheckIfApplicationExists(ctx context.Context, applicationID uuid.UUID) (bool, error)
	RevokeRefreshTokenByID(ctx context.Context, sessionID uuid.UUID) error
}

type Repository struct {
	repositories.ApplicationRepository
	repositories.RefreshTokenRepository
}

func NewRepository(q *pgstore.Queries) Repository {
	return Repository{
		ApplicationRepository: repositories.ApplicationRepository{Store: q},
		RefreshTokenRepository: repositories.RefreshTokenRepository{Store: q},
	}
}
