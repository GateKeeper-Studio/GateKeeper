package listusersessions

import (
	"context"
	"time"

	"github.com/gate-keeper/internal/infra/database/repositories"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	"github.com/google/uuid"
)

type IRepository interface {
	GetRefreshTokensByTenantUser(ctx context.Context, userID, tenantID uuid.UUID) (*Response, error)
}

type Repository struct {
	repositories.RefreshTokenRepository
}

func NewRepository(q *pgstore.Queries) Repository {
	return Repository{
		RefreshTokenRepository: repositories.RefreshTokenRepository{Store: q},
	}
}

func (r Repository) GetRefreshTokensByTenantUser(ctx context.Context, userID, tenantID uuid.UUID) (*Response, error) {
	tokens, err := r.RefreshTokenRepository.Store.GetRefreshTokensByTenantUser(ctx, pgstore.GetRefreshTokensByTenantUserParams{
		UserID:   userID,
		TenantID: tenantID,
	})

	if err != nil {
		return nil, err
	}

	now := time.Now()
	sessions := make([]SessionResponse, 0, len(tokens))

	for _, token := range tokens {
		expiresAt := token.ExpiresAt.Time
		createdAt := token.CreatedAt.Time
		sessions = append(sessions, SessionResponse{
			ID:        token.ID,
			UserID:    token.UserID,
			ExpiresAt: expiresAt.Format(time.RFC3339),
			CreatedAt: createdAt.Format(time.RFC3339),
			IsActive:  expiresAt.After(now),
		})
	}

	return &Response{Data: sessions}, nil
}
