package confirmuseremail

import (
	"context"
	"time"

	"github.com/gate-keeper/internal/domain/entities"
	"github.com/gate-keeper/internal/infra/database/repositories"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	"github.com/google/uuid"
	"github.com/jackc/pgx/v5/pgtype"
)

type IRepository interface {
	GetUserByEmail(ctx context.Context, userEmail string, applicationID uuid.UUID) (*entities.ApplicationUser, error)
	UpdateEmailConfirmation(ctx context.Context, emailConfirmation *entities.EmailConfirmation) error
	UpdateUser(ctx context.Context, user *entities.ApplicationUser) (*entities.ApplicationUser, error)
	AddAuthorizationCode(ctx context.Context, authorizationCode *entities.ApplicationAuthorizationCode) error
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

func (r Repository) AddAuthorizationCode(ctx context.Context, newAuthorizationCode *entities.ApplicationAuthorizationCode) error {
	err := r.Store.AddAuthorizationCode(ctx, pgstore.AddAuthorizationCodeParams{
		ID:                  newAuthorizationCode.ID,
		ApplicationID:       newAuthorizationCode.ApplicationID,
		UserID:              newAuthorizationCode.ApplicationUserId,
		ExpiredAt:           pgtype.Timestamp{Time: newAuthorizationCode.ExpiresAt, Valid: true},
		Code:                newAuthorizationCode.Code,
		RedirectUri:         newAuthorizationCode.RedirectUri,
		CodeChallenge:       newAuthorizationCode.CodeChallenge,
		CodeChallengeMethod: newAuthorizationCode.CodeChallengeMethod,
	})

	return err
}

func (r Repository) UpdateUser(ctx context.Context, user *entities.ApplicationUser) (*entities.ApplicationUser, error) {
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

func (r Repository) UpdateEmailConfirmation(ctx context.Context, emailConfirmation *entities.EmailConfirmation) error {
	err := r.Store.UpdateEmailConfirmation(ctx, pgstore.UpdateEmailConfirmationParams{
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

func (r Repository) GetUserByEmail(ctx context.Context, userEmail string, applicationID uuid.UUID) (*entities.ApplicationUser, error) {
	user, err := r.Store.GetUserByEmail(ctx, pgstore.GetUserByEmailParams{
		Email:         userEmail,
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
