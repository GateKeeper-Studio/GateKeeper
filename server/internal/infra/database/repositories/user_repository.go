package repositories

import (
	"context"
	"time"

	"github.com/gate-keeper/internal/domain/entities"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	"github.com/google/uuid"
	"github.com/jackc/pgx/v5/pgtype"
)

// IUserRepository defines all operations related to the TenantUser entity.
type IUserRepository interface {
	GetUserByID(ctx context.Context, userID uuid.UUID) (*entities.TenantUser, error)
	GetUserByEmail(ctx context.Context, userEmail string, applicationID uuid.UUID) (*entities.TenantUser, error)
	IsUserExistsByEmail(ctx context.Context, email string, applicationID uuid.UUID) (bool, error)
	AddUser(ctx context.Context, newUser *entities.TenantUser) error
	UpdateUser(ctx context.Context, user *entities.TenantUser) (*entities.TenantUser, error)
	DeleteTenantUser(ctx context.Context, applicationID, userID uuid.UUID) error
}

// UserRepository is the shared implementation for TenantUser-related DB operations.
type UserRepository struct {
	Store *pgstore.Queries
}

func (r UserRepository) GetUserByID(ctx context.Context, userID uuid.UUID) (*entities.TenantUser, error) {
	user, err := r.Store.GetUserById(ctx, userID)

	if err == ErrNoRows {
		return nil, nil
	}

	if err != nil {
		return nil, err
	}

	return &entities.TenantUser{
		ID:                 user.ID,
		Email:              user.Email,
		CreatedAt:          user.CreatedAt.Time,
		UpdatedAt:          user.UpdatedAt,
		IsActive:           user.IsActive,
		IsEmailConfirmed:   user.IsEmailConfirmed,
		TenantID:           user.TenantID,
		Preferred2FAMethod: user.Preferred2faMethod,
	}, nil
}

func (r UserRepository) GetUserByEmail(ctx context.Context, userEmail string, tenantID uuid.UUID) (*entities.TenantUser, error) {
	user, err := r.Store.GetUserByEmail(ctx, pgstore.GetUserByEmailParams{
		Email:    userEmail,
		TenantID: tenantID,
	})

	if err == ErrNoRows {
		return nil, nil
	}

	if err != nil {
		return nil, err
	}

	return &entities.TenantUser{
		ID:                 user.ID,
		Email:              user.Email,
		CreatedAt:          user.CreatedAt.Time,
		UpdatedAt:          user.UpdatedAt,
		IsActive:           user.IsActive,
		IsEmailConfirmed:   user.IsEmailConfirmed,
		TenantID:           user.TenantID,
		Preferred2FAMethod: user.Preferred2faMethod,
	}, nil
}

func (r UserRepository) IsUserExistsByEmail(ctx context.Context, email string, tenantID uuid.UUID) (bool, error) {
	_, err := r.Store.GetUserByEmail(ctx, pgstore.GetUserByEmailParams{
		Email:    email,
		TenantID: tenantID,
	})

	if err != nil {
		return false, nil
	}

	return true, nil
}

func (r UserRepository) AddUser(ctx context.Context, newUser *entities.TenantUser) error {
	return r.Store.AddUser(ctx, pgstore.AddUserParams{
		ID:               newUser.ID,
		Email:            newUser.Email,
		TenantID:         newUser.TenantID,
		CreatedAt:        pgtype.Timestamp{Time: newUser.CreatedAt, Valid: true},
		UpdatedAt:        newUser.UpdatedAt,
		IsActive:         newUser.IsActive,
		IsEmailConfirmed: newUser.IsEmailConfirmed,
	})
}

func (r UserRepository) UpdateUser(ctx context.Context, user *entities.TenantUser) (*entities.TenantUser, error) {
	now := time.Now().UTC()

	err := r.Store.UpdateUser(ctx, pgstore.UpdateUserParams{
		ID:                 user.ID,
		Email:              user.Email,
		UpdatedAt:          &now,
		IsActive:           user.IsActive,
		IsEmailConfirmed:   user.IsEmailConfirmed,
		Preferred2faMethod: user.Preferred2FAMethod,
	})

	return user, err
}

func (r UserRepository) DeleteTenantUser(ctx context.Context, tenantID, userID uuid.UUID) error {
	return r.Store.DeleteTenantUser(ctx, pgstore.DeleteTenantUserParams{
		ID:       userID,
		TenantID: tenantID,
	})
}
