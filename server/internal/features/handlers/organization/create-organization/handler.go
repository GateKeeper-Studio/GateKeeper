package createorganization

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
	newOrganization := entities.NewOrganization(command.Name, command.Description)

	if err := s.repository.AddOrganization(ctx, newOrganization); err != nil {
		return nil, err
	}

	return &Response{
		ID:          newOrganization.ID,
		Name:        newOrganization.Name,
		Description: newOrganization.Description,
		CreatedAt:   newOrganization.CreatedAt,
		UpdatedAt:   newOrganization.UpdatedAt,
	}, nil
}
