package beginwebauthnregistration

import (
	"context"
	"strings"

	"github.com/gate-keeper/internal/domain/entities"
	"github.com/gate-keeper/internal/infra/database/repositories"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	"github.com/google/uuid"
	"github.com/jackc/pgx/v5/pgtype"
)

type IRepository interface {
	GetApplicationByID(ctx context.Context, applicationID uuid.UUID) (*entities.Application, error)
	GetUserByID(ctx context.Context, userID uuid.UUID) (*entities.ApplicationUser, error)
	GetUserProfileByID(ctx context.Context, userID uuid.UUID) (*entities.UserProfile, error)
	GetMfaMethodByUserID(ctx context.Context, userID uuid.UUID, method string) (*entities.MfaMethod, error)
	AddMfaMethod(ctx context.Context, mfaMethod *entities.MfaMethod) error
	GetWebAuthnCredentialsByMfaMethodID(ctx context.Context, mfaMethodID uuid.UUID) ([]entities.MfaWebauthnCredentials, error)
	AddMfaWebauthnSession(ctx context.Context, session *entities.MfaWebauthnSession) error
}

type Repository struct {
	Store *pgstore.Queries
}

func (r Repository) GetApplicationByID(ctx context.Context, applicationID uuid.UUID) (*entities.Application, error) {
	application, err := r.Store.GetApplicationByID(ctx, applicationID)
	if err == repositories.ErrNoRows {
		return nil, nil
	}
	if err != nil {
		return nil, err
	}
	return &entities.Application{
		ID:                 application.ID,
		Name:               application.Name,
		Description:        application.Description,
		OrganizationID:     application.OrganizationID,
		CreatedAt:          application.CreatedAt.Time,
		IsActive:           application.IsActive,
		HasMfaAuthApp:      application.HasMfaAuthApp,
		HasMfaEmail:        application.HasMfaEmail,
		PasswordHashSecret: application.PasswordHashSecret,
		UpdatedAt:          application.UpdatedAt,
		Badges:             strings.Split(*application.Badges, ","),
		CanSelfSignUp:      application.CanSelfSignUp,
		CanSelfForgotPass:  application.CanSelfForgotPass,
	}, nil
}

func (r Repository) GetUserByID(ctx context.Context, userID uuid.UUID) (*entities.ApplicationUser, error) {
	user, err := r.Store.GetUserById(ctx, userID)
	if err == repositories.ErrNoRows {
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

func (r Repository) GetUserProfileByID(ctx context.Context, userID uuid.UUID) (*entities.UserProfile, error) {
	userProfile, err := r.Store.GetUserProfileByUserId(ctx, userID)
	if err == repositories.ErrNoRows {
		return nil, nil
	}
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

func (r Repository) GetMfaMethodByUserID(ctx context.Context, userID uuid.UUID, method string) (*entities.MfaMethod, error) {
	mfaMethod, err := r.Store.GetMfaMethodByUserIDAndMethod(ctx, pgstore.GetMfaMethodByUserIDAndMethodParams{
		UserID: userID,
		Type:   method,
	})
	if err == repositories.ErrNoRows {
		return nil, nil
	}
	if err != nil {
		return nil, err
	}
	return &entities.MfaMethod{
		ID:         mfaMethod.ID,
		UserID:     mfaMethod.UserID,
		Type:       mfaMethod.Type,
		Enabled:    mfaMethod.Enabled,
		CreatedAt:  mfaMethod.CreatedAt.Time,
		LastUsedAt: mfaMethod.LastUsedAt,
	}, nil
}

func (r Repository) AddMfaMethod(ctx context.Context, mfaMethod *entities.MfaMethod) error {
	return r.Store.AddMfaMethod(ctx, pgstore.AddMfaMethodParams{
		ID:         mfaMethod.ID,
		UserID:     mfaMethod.UserID,
		Type:       mfaMethod.Type,
		Enabled:    mfaMethod.Enabled,
		CreatedAt:  pgtype.Timestamp{Time: mfaMethod.CreatedAt, Valid: true},
		LastUsedAt: nil,
	})
}

func (r Repository) GetWebAuthnCredentialsByMfaMethodID(ctx context.Context, mfaMethodID uuid.UUID) ([]entities.MfaWebauthnCredentials, error) {
	rows, err := r.Store.GetMfaWebauthnCredentialsByMfaMethodID(ctx, mfaMethodID)
	if err != nil {
		return nil, err
	}
	creds := make([]entities.MfaWebauthnCredentials, 0, len(rows))
	for _, row := range rows {
		creds = append(creds, entities.MfaWebauthnCredentials{
			ID:           row.ID,
			MfaMethodID:  row.MfaMethodID,
			CredentialID: row.CredentialID,
			PublicKey:    row.PublicKey,
			SignCount:    uint32(row.SignCount),
			CreatedAt:    row.CreatedAt.Time,
		})
	}
	return creds, nil
}

func (r Repository) AddMfaWebauthnSession(ctx context.Context, session *entities.MfaWebauthnSession) error {
	return r.Store.AddMfaWebauthnSession(ctx, pgstore.AddMfaWebauthnSessionParams{
		ID:          session.ID,
		UserID:      session.UserID,
		SessionData: session.SessionData,
		CreatedAt:   pgtype.Timestamp{Time: session.CreatedAt, Valid: true},
		ExpiresAt:   pgtype.Timestamp{Time: session.ExpiresAt, Valid: true},
	})
}
