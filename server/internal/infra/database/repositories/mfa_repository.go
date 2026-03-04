package repositories

import (
	"context"

	"github.com/gate-keeper/internal/domain/entities"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	"github.com/google/uuid"
	"github.com/jackc/pgx/v5/pgtype"
)

// IMfaRepository defines all operations related to MFA methods, secrets, TOTP codes, email codes, WebAuthn credentials and sessions.
type IMfaRepository interface {
	GetMfaMethodByUserID(ctx context.Context, userID uuid.UUID, method string) (*entities.MfaMethod, error)
	AddMfaMethod(ctx context.Context, mfaMethod *entities.MfaMethod) error
	EnableMfaMethod(ctx context.Context, methodID uuid.UUID) error
	DisableMfaMethod(ctx context.Context, methodID uuid.UUID) error
	GetUserMfaMethods(ctx context.Context, userID uuid.UUID) ([]*entities.MfaMethod, error)
	GetMfaTotpSecretValidationByUserID(ctx context.Context, userID uuid.UUID) (*entities.MfaUserSecret, error)
	AddMfaTotpSecretValidation(ctx context.Context, mfaUserSecret *entities.MfaUserSecret) error
	UpdateMfaTotpSecretValidation(ctx context.Context, mfaUserSecret *entities.MfaUserSecret) error
	RevokeTotpSecretsByUserID(ctx context.Context, userID uuid.UUID) error
	AddMfaTotpCode(ctx context.Context, mfaTotpCode *entities.MfaTotpCode) error
	GetMfaTotpCodeByID(ctx context.Context, id uuid.UUID) (*entities.MfaTotpCode, error)
	DeleteMfaTotpCode(ctx context.Context, appMfaCodeID uuid.UUID) error
	AddMfaEmailCode(ctx context.Context, emailMfaCode *entities.MfaEmailCode) error
	GetMfaEmailCodeByToken(ctx context.Context, mfaMethodID uuid.UUID, token string) (*entities.MfaEmailCode, error)
	DeleteEmailMfaCodeByID(ctx context.Context, emailMfaCodeID uuid.UUID) error
	GetWebAuthnCredentialsByMfaMethodID(ctx context.Context, mfaMethodID uuid.UUID) ([]entities.MfaWebauthnCredentials, error)
	AddWebAuthnCredential(ctx context.Context, cred *entities.MfaWebauthnCredentials) error
	UpdateWebAuthnCredentialSignCount(ctx context.Context, credID uuid.UUID, signCount uint32) error
	AddMfaWebauthnSession(ctx context.Context, session *entities.MfaWebauthnSession) error
	GetMfaWebauthnSessionByID(ctx context.Context, id uuid.UUID) (*entities.MfaWebauthnSession, error)
	DeleteMfaWebauthnSession(ctx context.Context, id uuid.UUID) error
}

// MfaRepository is the shared implementation for MFA-related DB operations.
type MfaRepository struct {
	Store *pgstore.Queries
}

// --- MFA Methods ---

func (r MfaRepository) GetMfaMethodByUserID(ctx context.Context, userID uuid.UUID, method string) (*entities.MfaMethod, error) {
	mfaMethod, err := r.Store.GetMfaMethodByUserIDAndMethod(ctx, pgstore.GetMfaMethodByUserIDAndMethodParams{
		UserID: userID,
		Type:   method,
	})

	if err == ErrNoRows {
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

func (r MfaRepository) AddMfaMethod(ctx context.Context, mfaMethod *entities.MfaMethod) error {
	return r.Store.AddMfaMethod(ctx, pgstore.AddMfaMethodParams{
		ID:         mfaMethod.ID,
		UserID:     mfaMethod.UserID,
		Type:       mfaMethod.Type,
		Enabled:    mfaMethod.Enabled,
		CreatedAt:  pgtype.Timestamp{Time: mfaMethod.CreatedAt, Valid: true},
		LastUsedAt: nil,
	})
}

func (r MfaRepository) EnableMfaMethod(ctx context.Context, methodID uuid.UUID) error {
	return r.Store.EnableMfaMethod(ctx, methodID)
}

func (r MfaRepository) DisableMfaMethod(ctx context.Context, methodID uuid.UUID) error {
	return r.Store.DisableMfaMethod(ctx, methodID)
}

func (r MfaRepository) GetUserMfaMethods(ctx context.Context, userID uuid.UUID) ([]*entities.MfaMethod, error) {
	mfaMethods, err := r.Store.GetUserMfaMethods(ctx, userID)
	if err != nil {
		return nil, err
	}

	var result []*entities.MfaMethod
	for _, method := range mfaMethods {
		result = append(result, &entities.MfaMethod{
			ID:         method.ID,
			Type:       method.Type,
			UserID:     method.UserID,
			Enabled:    method.Enabled,
			CreatedAt:  method.CreatedAt.Time,
			LastUsedAt: method.LastUsedAt,
		})
	}

	return result, nil
}

// --- TOTP Secrets ---

func (r MfaRepository) GetMfaTotpSecretValidationByUserID(ctx context.Context, userID uuid.UUID) (*entities.MfaUserSecret, error) {
	mfaUserSecret, err := r.Store.GetMfaTotpSecretValidationByUserId(ctx, userID)

	if err == ErrNoRows {
		return nil, nil
	}

	if err != nil {
		return nil, err
	}

	return &entities.MfaUserSecret{
		ID:          mfaUserSecret.ID,
		UserID:      mfaUserSecret.UserID,
		Secret:      mfaUserSecret.Secret,
		IsValidated: mfaUserSecret.IsValidated,
		CreatedAt:   mfaUserSecret.CreatedAt.Time,
		ExpiresAt:   mfaUserSecret.ExpiresAt.Time,
	}, nil
}

func (r MfaRepository) AddMfaTotpSecretValidation(ctx context.Context, mfaUserSecret *entities.MfaUserSecret) error {
	return r.Store.AddMfaTotpSecretValidation(ctx, pgstore.AddMfaTotpSecretValidationParams{
		ID:          mfaUserSecret.ID,
		UserID:      mfaUserSecret.UserID,
		Secret:      mfaUserSecret.Secret,
		IsValidated: mfaUserSecret.IsValidated,
		CreatedAt:   pgtype.Timestamp{Time: mfaUserSecret.CreatedAt, Valid: true},
		ExpiresAt:   pgtype.Timestamp{Time: mfaUserSecret.ExpiresAt, Valid: true},
	})
}

func (r MfaRepository) UpdateMfaTotpSecretValidation(ctx context.Context, mfaUserSecret *entities.MfaUserSecret) error {
	return r.Store.UpdateMfaTotpSecretValidation(ctx, pgstore.UpdateMfaTotpSecretValidationParams{
		ID:          mfaUserSecret.ID,
		Secret:      mfaUserSecret.Secret,
		IsValidated: mfaUserSecret.IsValidated,
		CreatedAt:   pgtype.Timestamp{Time: mfaUserSecret.CreatedAt, Valid: true},
		ExpiresAt:   pgtype.Timestamp{Time: mfaUserSecret.ExpiresAt, Valid: true},
	})
}

func (r MfaRepository) RevokeTotpSecretsByUserID(ctx context.Context, userID uuid.UUID) error {
	return r.Store.RevokeMfaTotpSecretValidationFromUser(ctx, userID)
}

func (r MfaRepository) DeleteExpiredMfaTotpSecretValidationByUserID(ctx context.Context, userID uuid.UUID) error {
	return r.Store.DeleteExpiredMfaTotpSecretValidationByUserID(ctx, userID)
}

func (r MfaRepository) GetLastValidMfaTotpSecretByUserID(ctx context.Context, userID uuid.UUID) (*entities.MfaUserSecret, error) {
	row, err := r.Store.GetLastValidMfaTotpSecretByUserID(ctx, userID)
	if err != nil {
		return nil, err
	}

	return &entities.MfaUserSecret{
		ID:          row.ID,
		UserID:      row.UserID,
		Secret:      row.Secret,
		IsValidated: row.IsValidated,
		CreatedAt:   row.CreatedAt.Time,
		ExpiresAt:   row.ExpiresAt.Time,
	}, nil
}

// --- TOTP Codes ---

func (r MfaRepository) AddMfaTotpCode(ctx context.Context, mfaTotpCode *entities.MfaTotpCode) error {
	return r.Store.AddMfaTotpCode(ctx, pgstore.AddMfaTotpCodeParams{
		ID:          mfaTotpCode.ID,
		MfaMethodID: mfaTotpCode.MfaMethodID,
		Secret:      mfaTotpCode.Secret,
		CreatedAt:   pgtype.Timestamp{Time: mfaTotpCode.CreatedAt, Valid: true},
	})
}

func (r MfaRepository) GetMfaTotpCodeByID(ctx context.Context, id uuid.UUID) (*entities.MfaTotpCode, error) {
	appMfaCode, err := r.Store.GetMfaTotpCodeByID(ctx, id)

	if err == ErrNoRows {
		return nil, nil
	}

	if err != nil {
		return nil, err
	}

	return &entities.MfaTotpCode{
		ID:          appMfaCode.ID,
		MfaMethodID: appMfaCode.MfaMethodID,
		Secret:      appMfaCode.Secret,
		CreatedAt:   appMfaCode.CreatedAt.Time,
	}, nil
}

func (r MfaRepository) DeleteMfaTotpCode(ctx context.Context, appMfaCodeID uuid.UUID) error {
	return r.Store.DeleteMfaTotpCode(ctx, appMfaCodeID)
}

// --- Email MFA Codes ---

func (r MfaRepository) AddMfaEmailCode(ctx context.Context, emailMfaCode *entities.MfaEmailCode) error {
	return r.Store.AddMfaEmailCode(ctx, pgstore.AddMfaEmailCodeParams{
		ID:          emailMfaCode.ID,
		MfaMethodID: emailMfaCode.MfaMethodID,
		Token:       emailMfaCode.Token,
		CreatedAt:   pgtype.Timestamp{Time: emailMfaCode.CreatedAt, Valid: true},
		ExpiresAt:   pgtype.Timestamp{Time: emailMfaCode.ExpiresAt, Valid: true},
		Verified:    emailMfaCode.Verified,
	})
}

func (r MfaRepository) GetMfaEmailCodeByToken(ctx context.Context, mfaMethodID uuid.UUID, token string) (*entities.MfaEmailCode, error) {
	emailConfirmation, err := r.Store.GetMfaEmailCodeByToken(ctx, pgstore.GetMfaEmailCodeByTokenParams{
		Token:       token,
		MfaMethodID: mfaMethodID,
	})

	if err == ErrNoRows {
		return nil, nil
	}

	if err != nil {
		return nil, err
	}

	return &entities.MfaEmailCode{
		ID:          emailConfirmation.ID,
		MfaMethodID: emailConfirmation.MfaMethodID,
		Token:       emailConfirmation.Token,
		CreatedAt:   emailConfirmation.CreatedAt.Time,
		ExpiresAt:   emailConfirmation.ExpiresAt.Time,
		Verified:    emailConfirmation.Verified,
	}, nil
}

func (r MfaRepository) DeleteEmailMfaCodeByID(ctx context.Context, emailMfaCodeID uuid.UUID) error {
	return r.Store.DeleteMfaEmailCode(ctx, emailMfaCodeID)
}

// --- WebAuthn Credentials ---

func (r MfaRepository) GetWebAuthnCredentialsByMfaMethodID(ctx context.Context, mfaMethodID uuid.UUID) ([]entities.MfaWebauthnCredentials, error) {
	rows, err := r.Store.GetMfaWebauthnCredentialsByMfaMethodID(ctx, mfaMethodID)
	if err != nil {
		return nil, err
	}

	creds := make([]entities.MfaWebauthnCredentials, 0, len(rows))
	for _, row := range rows {
		creds = append(creds, entities.MfaWebauthnCredentials{
			ID:             row.ID,
			MfaMethodID:    row.MfaMethodID,
			CredentialID:   row.CredentialID,
			PublicKey:      row.PublicKey,
			SignCount:      uint32(row.SignCount),
			BackupEligible: row.BackupEligible,
			BackupState:    row.BackupState,
			CreatedAt:      row.CreatedAt.Time,
		})
	}

	return creds, nil
}

func (r MfaRepository) AddWebAuthnCredential(ctx context.Context, cred *entities.MfaWebauthnCredentials) error {
	return r.Store.AddMfaWebauthnCredential(ctx, pgstore.AddMfaWebauthnCredentialParams{
		ID:             cred.ID,
		MfaMethodID:    cred.MfaMethodID,
		CredentialID:   cred.CredentialID,
		PublicKey:      cred.PublicKey,
		SignCount:      int32(cred.SignCount),
		BackupEligible: cred.BackupEligible,
		BackupState:    cred.BackupState,
		CreatedAt:      pgtype.Timestamp{Time: cred.CreatedAt, Valid: true},
	})
}

func (r MfaRepository) UpdateWebAuthnCredentialSignCount(ctx context.Context, credID uuid.UUID, signCount uint32) error {
	return r.Store.UpdateMfaWebauthnCredentialSignCount(ctx, pgstore.UpdateMfaWebauthnCredentialSignCountParams{
		ID:        credID,
		SignCount: int32(signCount),
	})
}

// --- WebAuthn Sessions ---

func (r MfaRepository) AddMfaWebauthnSession(ctx context.Context, session *entities.MfaWebauthnSession) error {
	return r.Store.AddMfaWebauthnSession(ctx, pgstore.AddMfaWebauthnSessionParams{
		ID:          session.ID,
		UserID:      session.UserID,
		SessionData: session.SessionData,
		CreatedAt:   pgtype.Timestamp{Time: session.CreatedAt, Valid: true},
		ExpiresAt:   pgtype.Timestamp{Time: session.ExpiresAt, Valid: true},
	})
}

func (r MfaRepository) GetMfaWebauthnSessionByID(ctx context.Context, id uuid.UUID) (*entities.MfaWebauthnSession, error) {
	session, err := r.Store.GetMfaWebauthnSessionByID(ctx, id)

	if err == ErrNoRows {
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

func (r MfaRepository) DeleteMfaWebauthnSession(ctx context.Context, id uuid.UUID) error {
	return r.Store.DeleteMfaWebauthnSession(ctx, id)
}
