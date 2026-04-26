package createapplication

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
	newApplication := entities.AddApplication(
		command.Name,
		command.Description,
		command.TenantID,
		command.Badges,
		command.HasMfaEmail,
		command.HasMfaAuthApp,
		command.HasMfaWebauthn,
		true, // IsActive
		nil,  // UpdatedAt
		command.CanSelfSignUp,
		command.CanSelfForgotPass,
	)

	err := s.repository.AddApplication(ctx, newApplication)

	if err != nil {
		return nil, err
	}

	userRoleDescription := "Default user role"
	adminRoleDescription := "Default admin role"

	userRole := entities.NewApplicationRole(newApplication.ID, "User", &userRoleDescription)
	adminRole := entities.NewApplicationRole(newApplication.ID, "Admin", &adminRoleDescription)

	// Add default roles
	s.repository.AddRole(ctx, userRole)
	s.repository.AddRole(ctx, adminRole)

	return &Response{
		ID: newApplication.ID,
	}, nil
}
