package listusersessions

import (
	"context"
	"time"
	"github.com/gate-keeper/internal/infra/database/repositories"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	"github.com/google/uuid"
)

type IRepository interface {
	CheckIfApplicationExists(ctx context.Context, applicationID uuid.UUID) (bool, error)
	GetRefreshTokensByApplicationUser(ctx context.Context, userID, applicationID uuid.UUID) (*Response, error)
}

type Repository struct {
	repositories.ApplicationRepository
}

func NewRepository(q *pgstore.Queries) Repository {
	return Repository{
		ApplicationRepository: repositories.ApplicationRepository{Store: q},
	}
}

func (r Repository) GetRefreshTokensByApplicationUser(ctx context.Context, userID, applicationID uuid.UUID) (*Response, error) {
	tokens, err := r.ApplicationRepository.Store.GetRefreshTokensByApplicationUser(ctx, pgstore.GetRefreshTokensByApplicationUserParams{
		UserID:        userID,
		ApplicationID: applicationID,
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
