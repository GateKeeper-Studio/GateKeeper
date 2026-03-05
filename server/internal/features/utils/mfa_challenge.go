package application_utils

import (
	"context"
	"encoding/json"

	"github.com/gate-keeper/internal/domain/constants"
	"github.com/gate-keeper/internal/domain/entities"
	"github.com/gate-keeper/internal/domain/errors"
	mailservice "github.com/gate-keeper/internal/infra/mail-service"
	"github.com/google/uuid"
)

// MfaChallengeResult holds the data produced when an MFA challenge is created.
// The handler can use these fields to redirect the client for MFA completion.
type MfaChallengeResult struct {
	MfaType         string           `json:"mfaType"`
	MfaID           uuid.UUID        `json:"mfaId"`
	UserID          uuid.UUID        `json:"userId"`
	Email           string           `json:"email"`
	ApplicationID   uuid.UUID        `json:"applicationId"`
	WebAuthnOptions *json.RawMessage `json:"webAuthnOptions,omitempty"`
}

// IMfaChallengeRepository defines the repository methods needed to create MFA challenges.
type IMfaChallengeRepository interface {
	GetMfaMethodByUserID(ctx context.Context, userID uuid.UUID, method string) (*entities.MfaMethod, error)
	GetMfaTotpSecretValidationByUserID(ctx context.Context, userID uuid.UUID) (*entities.MfaUserSecret, error)
	AddMfaEmailCode(ctx context.Context, emailMfaCode *entities.MfaEmailCode) error
	AddMfaTotpCode(ctx context.Context, mfaTotpCode *entities.MfaTotpCode) error
	GetWebAuthnCredentialsByMfaMethodID(ctx context.Context, mfaMethodID uuid.UUID) ([]entities.MfaPasskeyCredentials, error)
	AddMfaPasskeySession(ctx context.Context, session *entities.MfaPasskeySession) error
	GetUserProfileByID(ctx context.Context, userID uuid.UUID) (*entities.UserProfile, error)
}

// CreateMfaChallenge creates the appropriate MFA challenge based on the user's
// preferred 2FA method. This is a shared utility used by both the login handler
// and OAuth callback handlers to avoid duplicating MFA challenge creation logic.
//
// Returns nil if the user has no preferred 2FA method.
func CreateMfaChallenge(
	ctx context.Context,
	repo IMfaChallengeRepository,
	mailSvc mailservice.IMailService,
	user *entities.TenantUser,
	applicationID uuid.UUID,
) (*MfaChallengeResult, error) {
	if user.Preferred2FAMethod == nil {
		return nil, nil
	}

	method := *user.Preferred2FAMethod

	switch method {
	case constants.MfaMethodEmail:
		return createEmailMfaChallenge(ctx, repo, mailSvc, user, applicationID)
	case constants.MfaMethodTotp:
		return createTotpMfaChallenge(ctx, repo, user, applicationID)
	case constants.MfaMethodWebauthn:
		return createWebAuthnMfaChallenge(ctx, repo, user, applicationID)
	default:
		return nil, nil
	}
}

func createEmailMfaChallenge(
	ctx context.Context,
	repo IMfaChallengeRepository,
	mailSvc mailservice.IMailService,
	user *entities.TenantUser,
	applicationID uuid.UUID,
) (*MfaChallengeResult, error) {
	userProfile, err := repo.GetUserProfileByID(ctx, user.ID)
	if err != nil {
		return nil, err
	}

	mfaMethod, err := repo.GetMfaMethodByUserID(ctx, user.ID, constants.MfaMethodEmail)
	if err != nil {
		return nil, err
	}
	if mfaMethod == nil {
		return nil, &errors.ErrMfaMethodNotFound
	}
	if !mfaMethod.Enabled {
		return nil, &errors.ErrMfaMethodNotEnabled
	}

	mfaEmailCode := entities.NewMfaEmailCode(mfaMethod.ID)
	if err := repo.AddMfaEmailCode(ctx, mfaEmailCode); err != nil {
		return nil, err
	}

	go func() {
		if err := mailSvc.SendMfaEmail(ctx, user.Email, userProfile.FirstName, mfaEmailCode.Token); err != nil {
			panic(err)
		}
	}()

	return &MfaChallengeResult{
		MfaType:       constants.MfaMethodEmail,
		MfaID:         mfaMethod.ID,
		UserID:        user.ID,
		Email:         user.Email,
		ApplicationID: applicationID,
	}, nil
}

func createTotpMfaChallenge(
	ctx context.Context,
	repo IMfaChallengeRepository,
	user *entities.TenantUser,
	applicationID uuid.UUID,
) (*MfaChallengeResult, error) {
	mfaMethod, err := repo.GetMfaMethodByUserID(ctx, user.ID, constants.MfaMethodTotp)
	if err != nil {
		return nil, err
	}
	if mfaMethod == nil {
		return nil, &errors.ErrMfaMethodNotFound
	}
	if !mfaMethod.Enabled {
		return nil, &errors.ErrMfaMethodNotEnabled
	}

	mfaTotpSecret, err := repo.GetMfaTotpSecretValidationByUserID(ctx, user.ID)
	if err != nil {
		return nil, err
	}
	if mfaTotpSecret == nil {
		return nil, &errors.ErrMfaUserSecretNotFound
	}

	mfaTotpCode := entities.NewMfaTotpCode(mfaMethod.ID, mfaTotpSecret.Secret)
	if err := repo.AddMfaTotpCode(ctx, mfaTotpCode); err != nil {
		return nil, err
	}

	return &MfaChallengeResult{
		MfaType:       constants.MfaMethodTotp,
		MfaID:         mfaTotpCode.ID,
		UserID:        user.ID,
		Email:         user.Email,
		ApplicationID: applicationID,
	}, nil
}

func createWebAuthnMfaChallenge(
	ctx context.Context,
	repo IMfaChallengeRepository,
	user *entities.TenantUser,
	applicationID uuid.UUID,
) (*MfaChallengeResult, error) {
	mfaMethod, err := repo.GetMfaMethodByUserID(ctx, user.ID, constants.MfaMethodWebauthn)
	if err != nil {
		return nil, err
	}
	if mfaMethod == nil {
		return nil, &errors.ErrMfaMethodNotFound
	}
	if !mfaMethod.Enabled {
		return nil, &errors.ErrMfaMethodNotEnabled
	}

	existingCreds, err := repo.GetWebAuthnCredentialsByMfaMethodID(ctx, mfaMethod.ID)
	if err != nil {
		return nil, err
	}
	if len(existingCreds) == 0 {
		return nil, &errors.ErrWebAuthnNoCredentials
	}

	userProfile, err := repo.GetUserProfileByID(ctx, user.ID)
	if err != nil {
		return nil, err
	}

	wa, err := NewWebAuthn()
	if err != nil {
		return nil, err
	}

	waUser := &WebAuthnUser{
		User:        user,
		Profile:     userProfile,
		Credentials: existingCreds,
	}

	credentialAssertion, sessionData, err := wa.BeginLogin(waUser)
	if err != nil {
		return nil, err
	}

	sessionDataJSON, err := json.Marshal(sessionData)
	if err != nil {
		return nil, err
	}

	webauthnSession, err := entities.NewMfaPasskeySession(user.ID, sessionDataJSON)
	if err != nil {
		return nil, err
	}

	if err := repo.AddMfaPasskeySession(ctx, webauthnSession); err != nil {
		return nil, err
	}

	optionsJSON, err := json.Marshal(credentialAssertion)
	if err != nil {
		return nil, err
	}

	options := json.RawMessage(optionsJSON)

	return &MfaChallengeResult{
		MfaType:         constants.MfaMethodWebauthn,
		MfaID:           webauthnSession.ID,
		UserID:          user.ID,
		Email:           user.Email,
		ApplicationID:   applicationID,
		WebAuthnOptions: &options,
	}, nil
}
