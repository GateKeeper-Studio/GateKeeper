package gettenantbyid

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
	tenant, err := s.repository.GetTenantByID(ctx, query.TenantID)

	if err != nil {
		return nil, err
	}

	return &Response{
		ID:          tenant.ID,
		Name:        tenant.Name,
		CreatedAt:   tenant.CreatedAt,
		UpdatedAt:   tenant.UpdatedAt,
		Description: tenant.Description,
	}, nil
}
