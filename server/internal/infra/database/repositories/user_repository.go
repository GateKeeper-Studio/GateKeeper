package repositories

import (
	"context"
	"time"

	"github.com/gate-keeper/internal/domain/entities"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	"github.com/google/uuid"
	"github.com/jackc/pgx/v5/pgtype"
)

// IUserRepository defines all operations related to the ApplicationUser entity.
type IUserRepository interface {
	GetUserByID(ctx context.Context, userID uuid.UUID) (*entities.ApplicationUser, error)
	GetUserByEmail(ctx context.Context, userEmail string, applicationID uuid.UUID) (*entities.ApplicationUser, error)
	IsUserExistsByEmail(ctx context.Context, email string, applicationID uuid.UUID) (bool, error)
	AddUser(ctx context.Context, newUser *entities.ApplicationUser) error
	UpdateUser(ctx context.Context, user *entities.ApplicationUser) (*entities.ApplicationUser, error)
	DeleteApplicationUser(ctx context.Context, applicationID, userID uuid.UUID) error
}

// UserRepository is the shared implementation for ApplicationUser-related DB operations.
type UserRepository struct {
	Store *pgstore.Queries
}

func (r UserRepository) GetUserByID(ctx context.Context, userID uuid.UUID) (*entities.ApplicationUser, error) {
	user, err := r.Store.GetUserById(ctx, userID)

	if err == ErrNoRows {
		return nil, nil
	}

	if err != nil {
		return nil, err
	}

	return &entities.ApplicationUser{
		ID:                 user.ID,
		Email:              user.Email,
		CreatedAt:          user.CreatedAt.Time,
		UpdatedAt:          user.UpdatedAt,
		IsActive:           user.IsActive,
		IsEmailConfirmed:   user.IsEmailConfirmed,
		ApplicationID:      user.ApplicationID,
		Preferred2FAMethod: user.Preferred2faMethod,
	}, nil
}

func (r UserRepository) GetUserByEmail(ctx context.Context, userEmail string, applicationID uuid.UUID) (*entities.ApplicationUser, error) {
	user, err := r.Store.GetUserByEmail(ctx, pgstore.GetUserByEmailParams{
		Email:         userEmail,
		ApplicationID: applicationID,
	})

	if err == ErrNoRows {
		return nil, nil
	}

	if err != nil {
		return nil, err
	}

	return &entities.ApplicationUser{
		ID:                 user.ID,
		Email:              user.Email,
		CreatedAt:          user.CreatedAt.Time,
		UpdatedAt:          user.UpdatedAt,
		IsActive:           user.IsActive,
		IsEmailConfirmed:   user.IsEmailConfirmed,
		ApplicationID:      user.ApplicationID,
		Preferred2FAMethod: user.Preferred2faMethod,
	}, nil
}

func (r UserRepository) IsUserExistsByEmail(ctx context.Context, email string, applicationID uuid.UUID) (bool, error) {
	_, err := r.Store.GetUserByEmail(ctx, pgstore.GetUserByEmailParams{
		Email:         email,
		ApplicationID: applicationID,
	})

	if err != nil {
		return false, nil
	}

	return true, nil
}

func (r UserRepository) AddUser(ctx context.Context, newUser *entities.ApplicationUser) error {
	return r.Store.AddUser(ctx, pgstore.AddUserParams{
		ID:               newUser.ID,
		Email:            newUser.Email,
		ApplicationID:    newUser.ApplicationID,
		CreatedAt:        pgtype.Timestamp{Time: newUser.CreatedAt, Valid: true},
		UpdatedAt:        newUser.UpdatedAt,
		IsActive:         newUser.IsActive,
		IsEmailConfirmed: newUser.IsEmailConfirmed,
	})
}

func (r UserRepository) UpdateUser(ctx context.Context, user *entities.ApplicationUser) (*entities.ApplicationUser, error) {
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

func (r UserRepository) DeleteApplicationUser(ctx context.Context, applicationID, userID uuid.UUID) error {
	return r.Store.DeleteApplicationUser(ctx, pgstore.DeleteApplicationUserParams{
		ID:            userID,
		ApplicationID: applicationID,
	})
}
