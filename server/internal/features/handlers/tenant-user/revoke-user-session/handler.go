package revokeusersession

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
	if err := s.repository.RevokeRefreshTokenByID(ctx, request.SessionID); err != nil {
		return err
	}

	return nil
}
