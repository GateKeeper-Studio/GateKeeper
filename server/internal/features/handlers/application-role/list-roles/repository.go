package listroles

import (
	"context"

	"github.com/gate-keeper/internal/infra/database/repositories"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	"github.com/google/uuid"
)

type IRepository interface {
	ListRolesFromApplicationPaged(ctx context.Context, applicationID uuid.UUID, limit, offset int) (*Response, error)
	CheckIfApplicationExists(ctx context.Context, applicationID uuid.UUID) (bool, error)
}

type Repository struct {
	Store *pgstore.Queries
}

func (r Repository) ListRolesFromApplicationPaged(ctx context.Context, applicationID uuid.UUID, limit, offset int) (*Response, error) {
	roles, err := r.Store.ListRolesFromApplicationPaged(ctx, pgstore.ListRolesFromApplicationPagedParams{
		ApplicationID: applicationID,
		Limit:         int32(limit),
		Offset:        int32(offset),
	})

	if err != nil && err != repositories.ErrNoRows {
		return nil, err
	}

	totalCount := 0

	if len(roles) > 0 {
		totalCount = int(roles[0].TotalCount)
	}

	result := Response{
		TotalCount: totalCount,
		Data:       []RoleResponse{},
	}

	for _, role := range roles {
		result.Data = append(result.Data, RoleResponse{
			ID:          role.ID,
			Name:        role.Name,
			Description: role.Description,
		})
	}

	return &result, nil
}

func (r Repository) CheckIfApplicationExists(ctx context.Context, applicationID uuid.UUID) (bool, error) {
	isApplicationExists, err := r.Store.CheckIfApplicationExists(ctx, applicationID)

	if err != nil {
		return false, err
	}

	return isApplicationExists, nil
}
