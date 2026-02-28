package verifywebauthnregistration

import (
	"context"

	"github.com/gate-keeper/internal/domain/entities"
	"github.com/gate-keeper/internal/infra/database/repositories"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	"github.com/google/uuid"
	"github.com/jackc/pgx/v5/pgtype"
)

type IRepository interface {
	GetMfaWebauthnSessionByID(ctx context.Context, id uuid.UUID) (*entities.MfaWebauthnSession, error)
	DeleteMfaWebauthnSession(ctx context.Context, id uuid.UUID) error
	GetUserByID(ctx context.Context, userID uuid.UUID) (*entities.ApplicationUser, error)
	GetUserProfileByID(ctx context.Context, userID uuid.UUID) (*entities.UserProfile, error)
	GetMfaMethodByUserID(ctx context.Context, userID uuid.UUID, method string) (*entities.MfaMethod, error)
	GetWebAuthnCredentialsByMfaMethodID(ctx context.Context, mfaMethodID uuid.UUID) ([]entities.MfaWebauthnCredentials, error)
	AddWebAuthnCredential(ctx context.Context, cred *entities.MfaWebauthnCredentials) error
	EnableMfaMethod(ctx context.Context, methodID uuid.UUID) error
}

type Repository struct {
	Store *pgstore.Queries
}

func (r Repository) GetMfaWebauthnSessionByID(ctx context.Context, id uuid.UUID) (*entities.MfaWebauthnSession, error) {
	session, err := r.Store.GetMfaWebauthnSessionByID(ctx, id)
	if err == repositories.ErrNoRows {
		return nil, nil
	}
	if err != nil {
		return nil, err
	}
	return &entities.MfaWebauthnSession{
		ID:          session.ID,
		UserID:      session.UserID,
		SessionData: session.SessionData,
		CreatedAt:   session.CreatedAt.Time,
		ExpiresAt:   session.ExpiresAt.Time,
	}, nil
}

func (r Repository) DeleteMfaWebauthnSession(ctx context.Context, id uuid.UUID) error {
	return r.Store.DeleteMfaWebauthnSession(ctx, id)
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

func (r Repository) AddWebAuthnCredential(ctx context.Context, cred *entities.MfaWebauthnCredentials) error {
	return r.Store.AddMfaWebauthnCredential(ctx, pgstore.AddMfaWebauthnCredentialParams{
		ID:           cred.ID,
		MfaMethodID:  cred.MfaMethodID,
		CredentialID: cred.CredentialID,
		PublicKey:    cred.PublicKey,
		SignCount:    int32(cred.SignCount),
		CreatedAt:    pgtype.Timestamp{Time: cred.CreatedAt, Valid: true},
	})
}

func (r Repository) EnableMfaMethod(ctx context.Context, methodID uuid.UUID) error {
	return r.Store.EnableMfaMethod(ctx, methodID)
}
