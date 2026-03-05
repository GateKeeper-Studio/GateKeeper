package verifypasskeyauth

import (
	"bytes"
	"context"
	"encoding/base64"
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

func New(q *pgstore.Queries) repositories.ServiceHandlerRs[Command, *Response] {
	return &Handler{
		repository: NewRepository(q),
	}
}

func (s *Handler) Handler(ctx context.Context, command Command) (*Response, error) {
	webauthnSession, err := s.repository.GetMfaPasskeySessionByID(ctx, command.SessionID)
	if err != nil {
		return nil, &errors.ErrWebAuthnSessionNotFound
	}
	if webauthnSession == nil {
		return nil, &errors.ErrWebAuthnSessionNotFound
	}
	if webauthnSession.IsExpired() {
		return nil, &errors.ErrWebAuthnSessionExpired
	}

	user, err := s.repository.GetUserByEmail(ctx, command.Email, command.ApplicationID)
	if err != nil {
		return nil, err
	}
	if user == nil {
		return nil, &errors.ErrUserNotFound
	}
	if !user.IsActive {
		return nil, &errors.ErrUserNotActive
	}
	if !user.IsEmailConfirmed {
		return nil, &errors.ErrEmailNotConfirmed
	}

	mfaMethod, err := s.repository.GetMfaMethodByUserID(ctx, user.ID, constants.MfaMethodWebauthn)
	if err != nil {
		return nil, err
	}
	if mfaMethod == nil || !mfaMethod.Enabled {
		return nil, &errors.ErrWebAuthnNotEnabled
	}

	existingCreds, err := s.repository.GetWebAuthnCredentialsByMfaMethodID(ctx, mfaMethod.ID)
	if err != nil {
		return nil, err
	}
	if len(existingCreds) == 0 {
		return nil, &errors.ErrWebAuthnNoCredentials
	}

	wa, err := application_utils.NewWebAuthn()
	if err != nil {
		return nil, err
	}

	userProfile, err := s.repository.GetUserProfileByID(ctx, user.ID)
	if err != nil {
		return nil, err
	}

	waUser := &application_utils.WebAuthnUser{
		User:        user,
		Profile:     userProfile,
		Credentials: existingCreds,
	}

	var sessionData webauthn.SessionData
	if err := json.Unmarshal([]byte(webauthnSession.SessionData), &sessionData); err != nil {
		return nil, err
	}

	parsedResponse, err := protocol.ParseCredentialRequestResponseBody(bytes.NewReader(command.AssertionData))
	if err != nil {
		return nil, &errors.ErrWebAuthnAuthenticationFailed
	}

	updatedCredential, err := wa.ValidateLogin(waUser, sessionData, parsedResponse)
	if err != nil {
		return nil, &errors.ErrWebAuthnAuthenticationFailed
	}

	// Update sign count for the used credential
	credIDBase64 := base64.StdEncoding.EncodeToString(updatedCredential.ID)
	for _, cred := range existingCreds {
		if cred.CredentialID == credIDBase64 {
			if err := s.repository.UpdateWebAuthnCredentialSignCount(ctx, cred.ID, updatedCredential.Authenticator.SignCount); err != nil {
				return nil, err
			}
			break
		}
	}

	if err := s.repository.DeleteMfaPasskeySession(ctx, command.SessionID); err != nil {
		return nil, err
	}

	authorizationSession, err := entities.CreateSessionCode(user.ID, command.ApplicationID)
	if err != nil {
		return nil, err
	}

	if err := s.repository.AddSessionCode(ctx, authorizationSession); err != nil {
		return nil, err
	}

	return &Response{
		SessionCode: authorizationSession.Token,
	}, nil
}
