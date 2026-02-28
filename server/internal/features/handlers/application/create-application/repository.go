package createapplication

import (
	"context"
	"strings"

	"github.com/gate-keeper/internal/domain/entities"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	"github.com/jackc/pgx/v5/pgtype"
)

type IRepository interface {
	AddApplication(ctx context.Context, application *entities.Application) error
	AddApplicationRole(ctx context.Context, role *entities.ApplicationRole) error
}

type Repository struct {
	Store *pgstore.Queries
}

func (r Repository) AddApplication(ctx context.Context, newApplication *entities.Application) error {
	badges := strings.Join(newApplication.Badges, ",")

	err := r.Store.AddApplication(ctx, pgstore.AddApplicationParams{
		ID:                 newApplication.ID,
		Name:               newApplication.Name,
		Description:        newApplication.Description,
		OrganizationID:     newApplication.OrganizationID,
		IsActive:           newApplication.IsActive,
		HasMfaAuthApp:      newApplication.HasMfaAuthApp,
		HasMfaEmail:        newApplication.HasMfaEmail,
		HasMfaWebauthn:     newApplication.HasMfaWebauthn,
		PasswordHashSecret: newApplication.PasswordHashSecret,
		Badges:             &badges,
		UpdatedAt:          newApplication.UpdatedAt,
		CanSelfSignUp:      newApplication.CanSelfSignUp,
		CanSelfForgotPass:  newApplication.CanSelfForgotPass,

		CreatedAt: pgtype.Timestamp{Time: newApplication.CreatedAt, Valid: true},
	})

	if err != nil {
		return err
	}

	return nil
}

func (r Repository) AddApplicationRole(ctx context.Context, newRole *entities.ApplicationRole) error {
	err := r.Store.AddRole(ctx, pgstore.AddRoleParams{
		ID:            newRole.ID,
		ApplicationID: newRole.ApplicationID,
		Name:          newRole.Name,
		Description:   newRole.Description,
		CreatedAt:     pgtype.Timestamp{Time: newRole.CreatedAt, Valid: true},
		UpdatedAt:     newRole.UpdatedAt,
	})

	return err
}
