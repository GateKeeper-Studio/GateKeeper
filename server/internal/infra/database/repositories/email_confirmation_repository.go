package repositories

import (
	"context"

	"github.com/gate-keeper/internal/domain/entities"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	"github.com/google/uuid"
	"github.com/jackc/pgx/v5/pgtype"
)

// IEmailConfirmationRepository defines all operations related to the EmailConfirmation entity.
type IEmailConfirmationRepository interface {
	GetEmailConfirmationByEmail(ctx context.Context, email string, userID uuid.UUID) (*entities.EmailConfirmation, error)
	AddEmailConfirmation(ctx context.Context, emailConfirmation *entities.EmailConfirmation) error
	UpdateEmailConfirmation(ctx context.Context, emailConfirmation *entities.EmailConfirmation) error
	DeleteEmailConfirmation(ctx context.Context, emailConfirmationID uuid.UUID) error
}

// EmailConfirmationRepository is the shared implementation for EmailConfirmation-related DB operations.
type EmailConfirmationRepository struct {
	Store *pgstore.Queries
}

func (r EmailConfirmationRepository) GetEmailConfirmationByEmail(ctx context.Context, email string, userID uuid.UUID) (*entities.EmailConfirmation, error) {
	emailConfirmation, err := r.Store.GetEmailConfirmationByEmail(ctx, pgstore.GetEmailConfirmationByEmailParams{
		Email:  email,
		UserID: userID,
	})

	if err == ErrNoRows {
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

func (r EmailConfirmationRepository) AddEmailConfirmation(ctx context.Context, emailConfirmation *entities.EmailConfirmation) error {
	return r.Store.AddEmailConfirmation(ctx, pgstore.AddEmailConfirmationParams{
		ID:        emailConfirmation.ID,
		UserID:    emailConfirmation.UserID,
		Email:     emailConfirmation.Email,
		Token:     emailConfirmation.Token,
		CreatedAt: pgtype.Timestamp{Time: emailConfirmation.CreatedAt, Valid: true},
		CoolDown:  pgtype.Timestamp{Time: emailConfirmation.CoolDown, Valid: true},
		ExpiresAt: pgtype.Timestamp{Time: emailConfirmation.ExpiresAt, Valid: true},
		IsUsed:    emailConfirmation.IsUsed,
	})
}

func (r EmailConfirmationRepository) UpdateEmailConfirmation(ctx context.Context, emailConfirmation *entities.EmailConfirmation) error {
	return r.Store.UpdateEmailConfirmation(ctx, pgstore.UpdateEmailConfirmationParams{
		ID:        emailConfirmation.ID,
		UserID:    emailConfirmation.UserID,
		Email:     emailConfirmation.Email,
		Token:     emailConfirmation.Token,
		CreatedAt: pgtype.Timestamp{Time: emailConfirmation.CreatedAt, Valid: true},
		CoolDown:  pgtype.Timestamp{Time: emailConfirmation.CoolDown, Valid: true},
		ExpiresAt: pgtype.Timestamp{Time: emailConfirmation.ExpiresAt, Valid: true},
		IsUsed:    emailConfirmation.IsUsed,
	})
}

func (r EmailConfirmationRepository) DeleteEmailConfirmation(ctx context.Context, emailConfirmationID uuid.UUID) error {
	return r.Store.DeleteEmailConfirmation(ctx, emailConfirmationID)
}
