package listtenantusers

import (
	"context"
	"encoding/json"

	"github.com/gate-keeper/internal/infra/database/repositories"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	"github.com/google/uuid"
)

type IRepository interface {
	GetUsersByTenantID(ctx context.Context, tenantID uuid.UUID, limit, offset int) (*Response, error)
}

type Repository struct {
	repositories.UserRepository
}

func NewRepository(q *pgstore.Queries) Repository {
	return Repository{
		UserRepository: repositories.UserRepository{Store: q},
	}
}

func (r Repository) GetUsersByTenantID(ctx context.Context, tenantID uuid.UUID, limit, offset int) (*Response, error) {
	users, err := r.UserRepository.Store.GetUsersByTenantID(ctx, pgstore.GetUsersByTenantIDParams{
		TenantID: tenantID,
		Limit:    int32(limit),
		Offset:   int32(offset),
	})

	if err != nil && err != repositories.ErrNoRows {
		return nil, err
	}

	totalCount := 0
	if len(users) > 0 {
		totalCount = int(users[0].TotalUsers)
	}

	result := Response{
		TotalCount: totalCount,
		Data:       []UserResponse{},
	}

	for _, user := range users {
		roles := []UserRole{}

		if err := json.Unmarshal(user.Roles, &roles); err != nil {
			return nil, err
		}

		displayName := ""
		if user.DisplayName != nil {
			displayName = *user.DisplayName
		}

		result.Data = append(result.Data, UserResponse{
			ID:          user.ID,
			DisplayName: displayName,
			Email:       user.Email,
			Roles:       roles,
		})
	}

	return &result, nil
}
