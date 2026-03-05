package listtenants

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
	tenants := make([]Response, 0)
	tenantsList, err := s.repository.ListTenants(ctx)

	if err != nil {
		return nil, err
	}

	for _, tenant := range *tenantsList {
		tenants = append(tenants, Response{
			ID:        tenant.ID,
			Name:      tenant.Name,
			CreatedAt: tenant.CreatedAt,
		})
	}

	return &tenants, nil
}
