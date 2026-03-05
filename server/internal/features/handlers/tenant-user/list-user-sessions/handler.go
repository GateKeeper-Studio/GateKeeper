package listusersessions

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

func (s *Handler) Handler(ctx context.Context, request Query) (*Response, error) {
	response, err := s.repository.GetRefreshTokensByTenantUser(ctx, request.UserID, request.TenantID)

	if err != nil {
		return nil, err
	}

	return response, nil
}
