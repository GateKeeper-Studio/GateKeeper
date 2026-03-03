package repositories

import (
	"context"

	"github.com/gate-keeper/internal/domain/entities"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	"github.com/google/uuid"
)

// IUserProfileRepository defines all operations related to the UserProfile entity.
type IUserProfileRepository interface {
	GetUserProfileByID(ctx context.Context, userID uuid.UUID) (*entities.UserProfile, error)
	AddUserProfile(ctx context.Context, newUserProfile *entities.UserProfile) error
	EditUserProfile(ctx context.Context, updatedUser *entities.UserProfile) error
}

// UserProfileRepository is the shared implementation for UserProfile-related DB operations.
type UserProfileRepository struct {
	Store *pgstore.Queries
}

func (r UserProfileRepository) GetUserProfileByID(ctx context.Context, userID uuid.UUID) (*entities.UserProfile, error) {
	userProfile, err := r.Store.GetUserProfileByUserId(ctx, userID)
	if err != nil {
		return nil, err
	}

	return &entities.UserProfile{
		UserID:      userProfile.UserID,
		DisplayName: userProfile.DisplayName,
		FirstName:   userProfile.FirstName,
		LastName:    userProfile.LastName,
		Address:     userProfile.Address,
		PhoneNumber: userProfile.PhoneNumber,
		PhotoURL:    userProfile.PhotoUrl,
	}, nil
}

func (r UserProfileRepository) AddUserProfile(ctx context.Context, newUserProfile *entities.UserProfile) error {
	return r.Store.AddUserProfile(ctx, pgstore.AddUserProfileParams{
		UserID:      newUserProfile.UserID,
		DisplayName: newUserProfile.DisplayName,
		FirstName:   newUserProfile.FirstName,
		LastName:    newUserProfile.LastName,
		Address:     newUserProfile.Address,
		PhoneNumber: newUserProfile.PhoneNumber,
		PhotoUrl:    newUserProfile.PhotoURL,
	})
}

func (r UserProfileRepository) EditUserProfile(ctx context.Context, newUserProfile *entities.UserProfile) error {
	return r.Store.UpdateUserProfile(ctx, pgstore.UpdateUserProfileParams{
		UserID:      newUserProfile.UserID,
		DisplayName: newUserProfile.DisplayName,
		FirstName:   newUserProfile.FirstName,
		LastName:    newUserProfile.LastName,
		Address:     newUserProfile.Address,
		PhoneNumber: newUserProfile.PhoneNumber,
		PhotoUrl:    newUserProfile.PhotoURL,
	})
}
