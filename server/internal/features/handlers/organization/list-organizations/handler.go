package listorganizations

import (
	"context"

	"github.com/gate-keeper/internal/infra/database/repositories"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
)

type Handler struct {
	repository IRepository
}

func New(q *pgstore.Queries) repositories.ServiceHandlerRs[Query, *[]Response] {
	return &Handler{
		repository: NewRepository(q),
	}
}

func (s *Handler) Handler(ctx context.Context, query Query) (*[]Response, error) {
	organizations := make([]Response, 0)
	organizationsList, err := s.repository.ListOrganizations(ctx)

	if err != nil {
		return nil, err
	}

	for _, organization := range *organizationsList {
		organizations = append(organizations, Response{
			ID:        organization.ID,
			Name:      organization.Name,
			CreatedAt: organization.CreatedAt,
		})
	}

	return &organizations, nil
}
