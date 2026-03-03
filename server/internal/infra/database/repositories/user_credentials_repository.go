package repositories

import (
	"context"

	"github.com/gate-keeper/internal/domain/entities"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	"github.com/google/uuid"
	"github.com/jackc/pgx/v5/pgtype"
)

// IUserCredentialsRepository defines all operations related to the UserCredentials entity.
type IUserCredentialsRepository interface {
	GetUserCredentialsByUserID(ctx context.Context, userID uuid.UUID) (*entities.UserCredentials, error)
	AddUserCredentials(ctx context.Context, userCredentials *entities.UserCredentials) error
	UpdateUserCredentials(ctx context.Context, userCredentials *entities.UserCredentials) error
}

// UserCredentialsRepository is the shared implementation for UserCredentials-related DB operations.
type UserCredentialsRepository struct {
	Store *pgstore.Queries
}

func (r UserCredentialsRepository) GetUserCredentialsByUserID(ctx context.Context, userID uuid.UUID) (*entities.UserCredentials, error) {
	userCredentials, err := r.Store.GetUserCredentialsByUserID(ctx, userID)

	if err == ErrNoRows {
		return nil, nil
	}

	if err != nil {
		return nil, err
	}

	return &entities.UserCredentials{
		ID:                userCredentials.ID,
		UserID:            userCredentials.UserID,
		PasswordAlgorithm: userCredentials.PasswordAlgorithm,
		PasswordHash:      userCredentials.PasswordHash,
		ShouldChangePass:  userCredentials.ShouldChangePass,
		CreatedAt:         userCredentials.CreatedAt.Time,
		UpdatedAt:         userCredentials.UpdatedAt,
	}, nil
}

func (r UserCredentialsRepository) AddUserCredentials(ctx context.Context, userCredentials *entities.UserCredentials) error {
	return r.Store.AddUserCredentials(ctx, pgstore.AddUserCredentialsParams{
		ID:                userCredentials.ID,
		UserID:            userCredentials.UserID,
		PasswordAlgorithm: userCredentials.PasswordAlgorithm,
		PasswordHash:      userCredentials.PasswordHash,
		ShouldChangePass:  userCredentials.ShouldChangePass,
		CreatedAt:         pgtype.Timestamp{Time: userCredentials.CreatedAt, Valid: true},
		UpdatedAt:         userCredentials.UpdatedAt,
	})
}

func (r UserCredentialsRepository) UpdateUserCredentials(ctx context.Context, userCredentials *entities.UserCredentials) error {
	return r.Store.UpdateUserCredentials(ctx, pgstore.UpdateUserCredentialsParams{
		UserID:            userCredentials.UserID,
		PasswordHash:      userCredentials.PasswordHash,
		PasswordAlgorithm: userCredentials.PasswordAlgorithm,
		ShouldChangePass:  userCredentials.ShouldChangePass,
		UpdatedAt:         userCredentials.UpdatedAt,
	})
}
