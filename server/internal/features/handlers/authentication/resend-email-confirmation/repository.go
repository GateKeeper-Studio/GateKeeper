package resendemailconfirmation

import (
	"context"

	"github.com/gate-keeper/internal/domain/entities"
	"github.com/gate-keeper/internal/infra/database/repositories"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	"github.com/google/uuid"
	"github.com/jackc/pgx/v5/pgtype"
)

type IRepository interface {
	GetUserProfileByID(ctx context.Context, userID uuid.UUID) (*entities.UserProfile, error)
	GetUserByEmail(ctx context.Context, userEmail string, applicationID uuid.UUID) (*entities.ApplicationUser, error)
	AddEmailConfirmation(ctx context.Context, emailConfirmation *entities.EmailConfirmation) error
	DeleteEmailConfirmation(ctx context.Context, emailConfirmationID uuid.UUID) error
	GetEmailConfirmationByEmail(ctx context.Context, email string, userID uuid.UUID) (*entities.EmailConfirmation, error)
}

type Repository struct {
	Store *pgstore.Queries
}

func (r Repository) GetEmailConfirmationByEmail(ctx context.Context, email string, userID uuid.UUID) (*entities.EmailConfirmation, error) {
	emailConfirmation, err := r.Store.GetEmailConfirmationByEmail(ctx, pgstore.GetEmailConfirmationByEmailParams{
		Email:  email,
		UserID: userID,
	})

	if err == repositories.ErrNoRows {
		return nil, nil
	}

	if err != nil {
		return nil, err
	}

	return &entities.EmailConfirmation{
		ID:        emailConfirmation.ID,
		UserID:    emailConfirmation.UserID,
		Email:     emailConfirmation.Email,
		Token:     emailConfirmation.Token,
		CreatedAt: emailConfirmation.CreatedAt.Time,
		CoolDown:  emailConfirmation.CoolDown.Time,
		ExpiresAt: emailConfirmation.ExpiresAt.Time,
		IsUsed:    emailConfirmation.IsUsed,
	}, nil
}

func (r Repository) DeleteEmailConfirmation(ctx context.Context, emailConfirmationID uuid.UUID) error {
	err := r.Store.DeleteEmailConfirmation(ctx, emailConfirmationID)

	return err
}

func (r Repository) AddEmailConfirmation(ctx context.Context, emailConfirmation *entities.EmailConfirmation) error {
	err := r.Store.AddEmailConfirmation(ctx, pgstore.AddEmailConfirmationParams{
		ID:        emailConfirmation.ID,
		UserID:    emailConfirmation.UserID,
		Email:     emailConfirmation.Email,
		Token:     emailConfirmation.Token,
		CreatedAt: pgtype.Timestamp{Time: emailConfirmation.CreatedAt, Valid: true},
		CoolDown:  pgtype.Timestamp{Time: emailConfirmation.CoolDown, Valid: true},
		ExpiresAt: pgtype.Timestamp{Time: emailConfirmation.ExpiresAt, Valid: true},
		IsUsed:    emailConfirmation.IsUsed,
	})

	return err
}

func (r Repository) GetUserByEmail(ctx context.Context, email string, applicationID uuid.UUID) (*entities.ApplicationUser, error) {
	user, err := r.Store.GetUserByEmail(ctx, pgstore.GetUserByEmailParams{
		Email:         email,
		ApplicationID: applicationID,
	})

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

func (r Repository) GetUserProfileByID(ctx context.Context, userID uuid.UUID) (*entities.UserProfile, error) {
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
