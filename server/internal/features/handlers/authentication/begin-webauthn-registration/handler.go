package beginwebauthnregistration

import (
	"context"
	"encoding/json"

	"github.com/gate-keeper/internal/domain/constants"
	"github.com/gate-keeper/internal/domain/entities"
	"github.com/gate-keeper/internal/domain/errors"
	application_utils "github.com/gate-keeper/internal/features/utils"
	"github.com/gate-keeper/internal/infra/database/repositories"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
)

type Handler struct {
	repository IRepository
}

func New(q *pgstore.Queries) repositories.ServiceHandlerRs[Command, *Response] {
	return &Handler{
		repository: Repository{Store: q},
	}
}

func (s *Handler) Handler(ctx context.Context, command Command) (*Response, error) {
	application, err := s.repository.GetApplicationByID(ctx, command.ApplicationID)
	if err != nil {
		return nil, err
	}
	if application == nil {
		return nil, &errors.ErrApplicationNotFound
	}

	user, err := s.repository.GetUserByID(ctx, command.UserID)
	if err != nil {
		return nil, err
	}
	if user == nil {
		return nil, &errors.ErrUserNotFound
	}

	userProfile, err := s.repository.GetUserProfileByID(ctx, command.UserID)
	if err != nil {
		return nil, err
	}

	mfaMethod, err := s.repository.GetMfaMethodByUserID(ctx, command.UserID, constants.MfaMethodWebauthn)
	if err != nil {
		return nil, err
	}

	if mfaMethod == nil {
		mfaMethod = entities.AddMfaMethod(command.UserID, constants.MfaMethodWebauthn)
		mfaMethod.Enabled = false
		if err := s.repository.AddMfaMethod(ctx, mfaMethod); err != nil {
			return nil, err
		}
	}

	existingCreds, err := s.repository.GetWebAuthnCredentialsByMfaMethodID(ctx, mfaMethod.ID)
	if err != nil {
		return nil, err
	}

	wa, err := application_utils.NewWebAuthn()
	if err != nil {
		return nil, err
	}

	waUser := &application_utils.WebAuthnUser{
		User:        user,
		Profile:     userProfile,
		Credentials: existingCreds,
	}

	credentialCreation, sessionData, err := wa.BeginRegistration(waUser)
	if err != nil {
		return nil, err
	}

	sessionDataJSON, err := json.Marshal(sessionData)
	if err != nil {
		return nil, err
	}

	webauthnSession, err := entities.NewMfaWebauthnSession(command.UserID, sessionDataJSON)
	if err != nil {
		return nil, err
	}

	if err := s.repository.AddMfaWebauthnSession(ctx, webauthnSession); err != nil {
		return nil, err
	}

	optionsJSON, err := json.Marshal(credentialCreation)
	if err != nil {
		return nil, err
	}

	return &Response{
		SessionID: webauthnSession.ID,
		Options:   json.RawMessage(optionsJSON),
	}, nil
}
