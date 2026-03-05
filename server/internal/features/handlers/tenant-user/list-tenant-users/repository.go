package listtenantusers

import (
	"context"
	"encoding/json"
	"github.com/gate-keeper/internal/infra/database/repositories"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	"github.com/google/uuid"
)

type IRepository interface {
	GetUsersByApplicationID(ctx context.Context, applicationID uuid.UUID, limit, offset int) (*Response, error)
	CheckIfApplicationExists(ctx context.Context, applicationID uuid.UUID) (bool, error)
}

type Repository struct {
	repositories.ApplicationRepository
}

func NewRepository(q *pgstore.Queries) Repository {
	return Repository{
		ApplicationRepository: repositories.ApplicationRepository{Store: q},
	}
}

func (r Repository) GetUsersByApplicationID(ctx context.Context, applicationID uuid.UUID, limit, offset int) (*Response, error) {
	users, err := r.ApplicationRepository.Store.GetUsersByApplicationID(ctx, pgstore.GetUsersByApplicationIDParams{
		ApplicationID: applicationID,
		Limit:         int32(limit),
		Offset:        int32(offset),
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
