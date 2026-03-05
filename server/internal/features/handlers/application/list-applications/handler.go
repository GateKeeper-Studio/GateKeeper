package listapplications

import (
	"context"

	"github.com/gate-keeper/internal/domain/errors"
	"github.com/gate-keeper/internal/infra/database/repositories"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
)

type Handler struct {
	Repository IRepository
}

func New(q *pgstore.Queries) repositories.ServiceHandlerRs[Query, *[]Response] {
	return &Handler{
		Repository: NewRepository(q),
	}
}

func (s *Handler) Handler(ctx context.Context, query Query) (*[]Response, error) {
	tenant, err := s.Repository.GetTenantByID(ctx, query.TenantID)

	if err != nil {
		return nil, err
	}

	if tenant == nil {
		return nil, &errors.ErrTenantNotFound
	}

	applications, err := s.Repository.ListApplicationsFromTenant(ctx, tenant.ID)

	if err != nil {
		return nil, err
	}

	response := make([]Response, 0)

	for _, application := range *applications {
		if len(application.Badges) == 1 && application.Badges[0] == "" {
			application.Badges = make([]string, 0)
		}

		response = append(response, Response{
			ID:          application.ID,
			Name:        application.Name,
			Description: application.Description,
			Badges:      application.Badges,
		})
	}

	return &response, nil
}
