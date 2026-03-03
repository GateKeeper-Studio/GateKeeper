package getorganizationbyid

import (
	"context"

	"github.com/gate-keeper/internal/infra/database/repositories"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
)

type Handler struct {
	repository IRepository
}

func New(q *pgstore.Queries) repositories.ServiceHandlerRs[Query, *Response] {
	return &Handler{
		repository: NewRepository(q),
	}
}

func (s *Handler) Handler(ctx context.Context, query Query) (*Response, error) {
	organization, err := s.repository.GetOrganizationByID(ctx, query.OrganizationID)

	if err != nil {
		return nil, err
	}

	return &Response{
		ID:          organization.ID,
		Name:        organization.Name,
		CreatedAt:   organization.CreatedAt,
		UpdatedAt:   organization.UpdatedAt,
		Description: organization.Description,
	}, nil
}
