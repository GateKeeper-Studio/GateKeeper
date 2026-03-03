package createrole

import (
	"context"

	"github.com/gate-keeper/internal/domain/entities"
	"github.com/gate-keeper/internal/domain/errors"
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

func (s *Handler) Handler(ctx context.Context, request Command) (*Response, error) {
	isApplicationExists, err := s.repository.CheckIfApplicationExists(ctx, request.ApplicationID)

	if err != nil {
		return nil, err
	}

	if !isApplicationExists {
		return nil, &errors.ErrApplicationNotFound
	}

	newRole := entities.NewApplicationRole(request.ApplicationID, request.Name, request.Description)

	if err := s.repository.AddRole(ctx, newRole); err != nil {
		return nil, err
	}

	return &Response{
		ID:            newRole.ID,
		Name:          newRole.Name,
		Description:   newRole.Description,
		ApplicationID: newRole.ApplicationID,
		CreatedAt:     newRole.CreatedAt,
		UpdatedAt:     newRole.UpdatedAt,
	}, nil
}
