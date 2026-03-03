package removeapplication

import (
	"context"

	"github.com/gate-keeper/internal/infra/database/repositories"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
)

type Handler struct {
	Repository IRepository
}

func New(q *pgstore.Queries) repositories.ServiceHandler[Command] {
	return &Handler{
		Repository: NewRepository(q),
	}
}

func (s *Handler) Handler(ctx context.Context, command Command) error {
	err := s.Repository.RemoveApplication(ctx, command.ApplicationID)

	if err != nil {
		return err
	}

	return nil
}
