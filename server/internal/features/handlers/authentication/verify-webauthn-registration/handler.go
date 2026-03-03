package verifywebauthnregistration

import (
	"bytes"
	"context"
	"encoding/json"

	"github.com/gate-keeper/internal/domain/constants"
	"github.com/gate-keeper/internal/domain/entities"
	"github.com/gate-keeper/internal/domain/errors"
	application_utils "github.com/gate-keeper/internal/features/utils"
	"github.com/gate-keeper/internal/infra/database/repositories"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	"github.com/go-webauthn/webauthn/protocol"
	"github.com/go-webauthn/webauthn/webauthn"
)

type Handler struct {
	repository IRepository
}

func New(q *pgstore.Queries) repositories.ServiceHandler[Command] {
	return &Handler{
		repository: NewRepository(q),
	}
}

func (s *Handler) Handler(ctx context.Context, command Command) error {
	webauthnSession, err := s.repository.GetMfaWebauthnSessionByID(ctx, command.SessionID)
	if err != nil {
		return &errors.ErrWebAuthnSessionNotFound
	}
	if webauthnSession == nil {
		return &errors.ErrWebAuthnSessionNotFound
	}
	if webauthnSession.IsExpired() {
		return &errors.ErrWebAuthnSessionExpired
	}

	user, err := s.repository.GetUserByID(ctx, command.UserID)
	if err != nil {
		return err
	}
	if user == nil {
		return &errors.ErrUserNotFound
	}

	userProfile, err := s.repository.GetUserProfileByID(ctx, command.UserID)
	if err != nil {
		return err
	}

	mfaMethod, err := s.repository.GetMfaMethodByUserID(ctx, command.UserID, constants.MfaMethodWebauthn)
	if err != nil {
		return err
	}
	if mfaMethod == nil {
		return &errors.ErrWebAuthnNotEnabled
	}

	existingCreds, err := s.repository.GetWebAuthnCredentialsByMfaMethodID(ctx, mfaMethod.ID)
	if err != nil {
		return err
	}

	wa, err := application_utils.NewWebAuthn()
	if err != nil {
		return err
	}

	waUser := &application_utils.WebAuthnUser{
		User:        user,
		Profile:     userProfile,
		Credentials: existingCreds,
	}

	var sessionData webauthn.SessionData
	if err := json.Unmarshal([]byte(webauthnSession.SessionData), &sessionData); err != nil {
		return err
	}

	parsedResponse, err := protocol.ParseCredentialCreationResponseBody(bytes.NewReader(command.CredentialData))
	if err != nil {
		return &errors.ErrWebAuthnRegistrationFailed
	}

	credential, err := wa.CreateCredential(waUser, sessionData, parsedResponse)
	if err != nil {
		return &errors.ErrWebAuthnRegistrationFailed
	}

	newCred, err := entities.NewMfaWebauthnCredentials(
		mfaMethod.ID,
		credential.ID,
		credential.PublicKey,
		credential.Authenticator.SignCount,
	)
	if err != nil {
		return err
	}

	if err := s.repository.AddWebAuthnCredential(ctx, newCred); err != nil {
		return err
	}

	if !mfaMethod.Enabled {
		if err := s.repository.EnableMfaMethod(ctx, mfaMethod.ID); err != nil {
			return err
		}
	}

	return s.repository.DeleteMfaWebauthnSession(ctx, command.SessionID)
}
