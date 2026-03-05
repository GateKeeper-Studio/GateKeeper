package createtenant

import (
	"context"

	"github.com/gate-keeper/internal/domain/entities"
	"github.com/gate-keeper/internal/infra/database/repositories"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
)

type Handler struct {
	repository IRepository
}

func New(q *pgstore.Queries) repositories.ServiceHandlerRs[Command, *Response] {
	return &Handler{
		repository: NewRepository(q),
	}
}

func (s *Handler) Handler(ctx context.Context, command Command) (*Response, error) {
	newTenant := entities.NewTenant(command.Name, command.Description)

	if err := s.repository.AddTenant(ctx, newTenant); err != nil {
		return nil, err
	}

	return &Response{
		ID:          newTenant.ID,
		Name:        newTenant.Name,
		Description: newTenant.Description,
		CreatedAt:   newTenant.CreatedAt,
		UpdatedAt:   newTenant.UpdatedAt,
	}, nil
}
