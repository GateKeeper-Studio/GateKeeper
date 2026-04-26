package edittenant

import (
	"context"
	"time"

	"github.com/gate-keeper/internal/domain/errors"
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

func (s *Handler) Handler(ctx context.Context, command Command) error {
	tenant, err := s.repository.GetTenantByID(ctx, command.ID)

	if err != nil {
		return err
	}

	if tenant == nil {
		return &errors.ErrTenantNotFound
	}

	utcNow := time.Now().UTC()

	tenant.Name = command.Name
	tenant.Description = command.Description
	if command.PasswordHashSecret != nil {
		tenant.PasswordHashSecret = *command.PasswordHashSecret
	}
	tenant.UpdatedAt = &utcNow

	if err := s.repository.UpdateTenant(ctx, tenant); err != nil {
		return err
	}

	return nil
}
