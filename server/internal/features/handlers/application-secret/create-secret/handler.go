package createsecret

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

	newSecret := entities.NewApplicationSecret(request.ApplicationID, request.Name, request.ExpiresAt)

	if err := s.repository.AddSecret(ctx, newSecret); err != nil {
		return nil, err
	}

	return &Response{
		ID:            newSecret.ID,
		ApplicationID: newSecret.ApplicationID,
		Name:          newSecret.Name,
		Value:         newSecret.Value,
		CreatedAt:     newSecret.CreatedAt,
		UpdatedAt:     newSecret.UpdatedAt,
		ExpiresAt:     newSecret.ExpiresAt,
	}, nil
}
