package repositories

import (
	"context"

	"github.com/gate-keeper/internal/domain/entities"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	"github.com/google/uuid"
	"github.com/jackc/pgx/v5/pgtype"
)

// IRoleRepository defines all operations related to the ApplicationRole and UserRole entities.
type IRoleRepository interface {
	AddRole(ctx context.Context, newRole *entities.ApplicationRole) error
	RemoveRole(ctx context.Context, roleID uuid.UUID) error
	ListRolesFromApplication(ctx context.Context, applicationID uuid.UUID) (*[]entities.ApplicationRole, error)
	GetRolesByUserID(ctx context.Context, userID uuid.UUID) ([]entities.ApplicationRole, error)
	AddUserRole(ctx context.Context, newUserRole *entities.UserRole) error
	RemoveUserRole(ctx context.Context, userRole *entities.UserRole) error
}

// RoleRepository is the shared implementation for ApplicationRole/UserRole-related DB operations.
type RoleRepository struct {
	Store *pgstore.Queries
}

func (r RoleRepository) AddRole(ctx context.Context, newRole *entities.ApplicationRole) error {
	return r.Store.AddRole(ctx, pgstore.AddRoleParams{
		ID:            newRole.ID,
		ApplicationID: newRole.ApplicationID,
		Name:          newRole.Name,
		Description:   newRole.Description,
		CreatedAt:     pgtype.Timestamp{Time: newRole.CreatedAt, Valid: true},
		UpdatedAt:     newRole.UpdatedAt,
	})
}

func (r RoleRepository) RemoveRole(ctx context.Context, roleID uuid.UUID) error {
	return r.Store.RemoveRole(ctx, roleID)
}

func (r RoleRepository) ListRolesFromApplication(ctx context.Context, applicationID uuid.UUID) (*[]entities.ApplicationRole, error) {
	roles, err := r.Store.ListRolesFromApplication(ctx, applicationID)

	if err != nil && err != ErrNoRows {
		return nil, err
	}

	var applicationRoles []entities.ApplicationRole
	for _, role := range roles {
		applicationRoles = append(applicationRoles, entities.ApplicationRole{
			ID:            role.ID,
			ApplicationID: role.ApplicationID,
			Name:          role.Name,
			Description:   role.Description,
			CreatedAt:     role.CreatedAt.Time,
			UpdatedAt:     role.UpdatedAt,
		})
	}

	return &applicationRoles, nil
}

func (r RoleRepository) GetRolesByUserID(ctx context.Context, userID uuid.UUID) ([]entities.ApplicationRole, error) {
	roles, err := r.Store.GetUserRoles(ctx, userID)
	if err != nil {
		return nil, err
	}

	var applicationRoles []entities.ApplicationRole
	for _, role := range roles {
		applicationRoles = append(applicationRoles, entities.ApplicationRole{
			ID:   role.ID,
			Name: role.Name,
		})
	}

	return applicationRoles, nil
}

func (r RoleRepository) AddUserRole(ctx context.Context, newUserRole *entities.UserRole) error {
	return r.Store.AddUserRole(ctx, pgstore.AddUserRoleParams{
		UserID:    newUserRole.UserID,
		RoleID:    newUserRole.RoleID,
		CreatedAt: pgtype.Timestamp{Time: newUserRole.CreatedAt, Valid: true},
	})
}

func (r RoleRepository) RemoveUserRole(ctx context.Context, userRole *entities.UserRole) error {
	return r.Store.RemoveUserRole(ctx, pgstore.RemoveUserRoleParams{
		UserID: userRole.UserID,
		RoleID: userRole.RoleID,
	})
}
