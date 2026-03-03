package login

import (
	"context"
	"encoding/json"

	"github.com/gate-keeper/internal/domain/constants"
	"github.com/gate-keeper/internal/domain/entities"
	"github.com/gate-keeper/internal/domain/errors"
	application_utils "github.com/gate-keeper/internal/features/utils"
	"github.com/gate-keeper/internal/infra/database/repositories"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	mailservice "github.com/gate-keeper/internal/infra/mail-service"
)

type Handler struct {
	repository  IRepository
	mailService mailservice.IMailService
}

func New(q *pgstore.Queries) repositories.ServiceHandlerRs[Command, *Response] {
	return &Handler{
		repository:  NewRepository(q),
		mailService: &mailservice.MailService{},
	}
}

func (s *Handler) Handler(ctx context.Context, command Command) (*Response, error) {
	user, err := s.repository.GetUserByEmail(ctx, command.Email, command.ApplicationID)

	if err != nil {
		return nil, &errors.ErrUserNotFound
	}

	if user == nil {
		return nil, &errors.ErrUserNotFound
	}

	if !user.IsActive {
		return nil, &errors.ErrUserNotActive
	}

	userCredentials, err := s.repository.GetUserCredentialsByUserID(ctx, user.ID)

	if err != nil {
		return nil, err
	}

	isPasswordCorrect, err := application_utils.ComparePassword(userCredentials.PasswordHash, command.Password)

	if err != nil {
		return nil, err
	}

	if !isPasswordCorrect {
		return nil, &errors.ErrEmailOrPasswordInvalid
	}

	if !user.IsEmailConfirmed {
		return nil, &errors.ErrEmailNotConfirmed
	}

	// Revoke all Password change codes if exists
	if err := s.repository.RevokeAllChangePasswordCodeByUserID(ctx, user.ID); err != nil {
		return nil, err
	}

	var changePasswordCode *entities.ChangePasswordCode = nil

	// If the user should change their password, create a new change password code
	if userCredentials.ShouldChangePass {
		changePasswordCode = entities.NewChangePasswordCode(user.ID, user.Email)

		if err := s.repository.AddChangePasswordCode(ctx, changePasswordCode); err != nil {
			return nil, err
		}
	}

	if user.Preferred2FAMethod != nil && *user.Preferred2FAMethod == constants.MfaMethodEmail {
		userProfile, err := s.repository.GetUserProfileByID(ctx, user.ID)

		if err != nil {
			return nil, err
		}

		mfaMethod, err := s.repository.GetMfaMethodByUserID(ctx, user.ID, *user.Preferred2FAMethod)

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

		if err := s.repository.AddMfaEmailCode(ctx, mfaEmailCode); err != nil {
			return nil, err
		}

		go func() {
			if err := s.mailService.SendMfaEmail(ctx, user.Email, userProfile.FirstName, mfaEmailCode.Token); err != nil {
				panic(err)
			}
		}()

		response := &Response{
			MfaType:            user.Preferred2FAMethod,
			ChangePasswordCode: nil,
			Message:            "MFA is required, please enter the code that we sent yo your e-mail",
			SessionCode:        nil,
			UserID:             user.ID,
			MfaID:              &mfaMethod.ID,
		}

		if changePasswordCode != nil {
			response.ChangePasswordCode = &changePasswordCode.Token
		}

		return response, nil
	}
	// #endregion

	if user.Preferred2FAMethod != nil && *user.Preferred2FAMethod == constants.MfaMethodTotp {
		mfaMethod, err := s.repository.GetMfaMethodByUserID(ctx, user.ID, *user.Preferred2FAMethod)

		if err != nil {
			return nil, err
		}

		if mfaMethod == nil {
			return nil, &errors.ErrMfaMethodNotFound
		}

		if !mfaMethod.Enabled {
			return nil, &errors.ErrMfaMethodNotEnabled
		}

		mfaTotpSecretValidation, err := s.repository.GetMfaTotpSecretValidationByUserID(ctx, user.ID)

		if err != nil {
			return nil, err
		}

		if mfaTotpSecretValidation == nil {
			return nil, &errors.ErrMfaUserSecretNotFound
		}

		mfaTotpCode := entities.NewMfaTotpCode(mfaMethod.ID, mfaTotpSecretValidation.Secret)

		if err := s.repository.AddMfaTotpCode(ctx, mfaTotpCode); err != nil {
			return nil, err
		}

		response := &Response{
			MfaType:            user.Preferred2FAMethod,
			ChangePasswordCode: nil,
			Message:            "MFA is required, please enter the code from your authentication app",
			SessionCode:        nil,
			UserID:             user.ID,
			MfaID:              &mfaTotpCode.ID,
		}

		if changePasswordCode != nil {
			response.ChangePasswordCode = &changePasswordCode.Token
		}

		return response, nil
	}

	if user.Preferred2FAMethod != nil && *user.Preferred2FAMethod == constants.MfaMethodWebauthn {
		mfaMethod, err := s.repository.GetMfaMethodByUserID(ctx, user.ID, *user.Preferred2FAMethod)
		if err != nil {
			return nil, err
		}
		if mfaMethod == nil {
			return nil, &errors.ErrMfaMethodNotFound
		}
		if !mfaMethod.Enabled {
			return nil, &errors.ErrMfaMethodNotEnabled
		}

		existingCreds, err := s.repository.GetWebAuthnCredentialsByMfaMethodID(ctx, mfaMethod.ID)
		if err != nil {
			return nil, err
		}
		if len(existingCreds) == 0 {
			return nil, &errors.ErrWebAuthnNoCredentials
		}

		userProfile, err := s.repository.GetUserProfileByID(ctx, user.ID)
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

		credentialAssertion, sessionData, err := wa.BeginLogin(waUser)
		if err != nil {
			return nil, err
		}

		sessionDataJSON, err := json.Marshal(sessionData)
		if err != nil {
			return nil, err
		}

		webauthnSession, err := entities.NewMfaWebauthnSession(user.ID, sessionDataJSON)
		if err != nil {
			return nil, err
		}

		if err := s.repository.AddMfaWebauthnSession(ctx, webauthnSession); err != nil {
			return nil, err
		}

		optionsJSON, err := json.Marshal(credentialAssertion)
		if err != nil {
			return nil, err
		}

		options := json.RawMessage(optionsJSON)
		response := &Response{
			MfaType:         user.Preferred2FAMethod,
			Message:         "MFA is required, please complete the WebAuthn challenge",
			SessionCode:     nil,
			UserID:          user.ID,
			MfaID:           &webauthnSession.ID,
			WebAuthnOptions: &options,
		}

		if changePasswordCode != nil {
			response.ChangePasswordCode = &changePasswordCode.Token
		}

		return response, nil
	}

	sessionToken, err := entities.CreateSessionCode(
		user.ID,
		command.ApplicationID,
	)

	if err != nil {
		return nil, err
	}

	if err := s.repository.AddSessionCode(ctx, sessionToken); err != nil {
		return nil, err
	}

	tokenString := sessionToken.Token

	if changePasswordCode == nil {
		return &Response{
			MfaType:            nil,
			Message:            "Login successful",
			ChangePasswordCode: nil,
			SessionCode:        &tokenString,
			UserID:             user.ID,
		}, nil
	}

	return &Response{
		MfaType:            nil,
		Message:            "Login successful",
		ChangePasswordCode: &changePasswordCode.Token,
		SessionCode:        &tokenString,
		UserID:             user.ID,
	}, nil
}
