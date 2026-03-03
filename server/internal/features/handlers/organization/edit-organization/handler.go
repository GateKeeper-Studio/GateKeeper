package editorganization

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
	organization, err := s.repository.GetOrganizationByID(ctx, command.ID)

	if err != nil {
		return err
	}

	if organization == nil {
		return &errors.ErrOrganizationNotFound
	}

	utcNow := time.Now().UTC()

	organization.Name = command.Name
	organization.Description = command.Description
	organization.UpdatedAt = &utcNow

	if err := s.repository.UpdateOrganization(ctx, organization); err != nil {
		return err
	}

	return nil
}
