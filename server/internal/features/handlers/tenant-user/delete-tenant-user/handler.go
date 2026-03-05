package deletetenantuser

import (
	"context"

	"github.com/gate-keeper/internal/infra/database/repositories"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
)

type Handler struct {
	repository IRepository
}

func New(q *pgstore.Queries) repositories.ServiceHandler[Command] {
	return &Handler{
		repository: NewRepository(q),
	}
}

func (s *Handler) Handler(ctx context.Context, request Command) error {
	if err := s.repository.DeleteTenantUser(ctx, request.TenantID, request.UserID); err != nil {
		return err
	}

	return nil
}
