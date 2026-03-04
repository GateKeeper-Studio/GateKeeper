package reauthenticate

import (
	"context"

	"github.com/gate-keeper/internal/domain/constants"
	"github.com/gate-keeper/internal/domain/entities"
	"github.com/gate-keeper/internal/domain/errors"
	application_utils "github.com/gate-keeper/internal/features/utils"
	"github.com/gate-keeper/internal/infra/database/repositories"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	"github.com/pquerna/otp/totp"
)

type Handler struct {
	repository IRepository
}

func New(q *pgstore.Queries) repositories.ServiceHandlerRs[Command, *Response] {
	return &Handler{
		repository: NewRepository(q),
	}
}

func (h *Handler) Handler(ctx context.Context, command Command) (*Response, error) {
	user, err := h.repository.GetUserByID(ctx, command.UserID)
	if err != nil {
		return nil, err
	}
	if user == nil {
		return nil, &errors.ErrUserNotFound
	}

	userCredentials, err := h.repository.GetUserCredentialsByUserID(ctx, user.ID)
	if err != nil {
		return nil, err
	}
	if userCredentials == nil {
		return nil, &errors.ErrUserCredentialsNotFound
	}

	// Verify current password
	isPasswordCorrect, err := application_utils.ComparePassword(userCredentials.PasswordHash, command.Password)
	if err != nil {
		return nil, err
	}

	if !isPasswordCorrect {
		// Log failed reauthentication
		failureDetails := "incorrect password"
		auditLog := entities.NewAuditLog(user.ID, command.ApplicationID, constants.AuditEventFailedReauth, "", "", "failure", &failureDetails)
		_ = h.repository.AddAuditLog(ctx, auditLog)
		return nil, &errors.ErrCurrentPasswordIncorrect
	}

	// If user has TOTP MFA enabled, verify the TOTP code
	if user.Preferred2FAMethod != nil && *user.Preferred2FAMethod == constants.MfaMethodTotp {
		if command.TOTPCode == nil || *command.TOTPCode == "" {
			return nil, &errors.ErrMfaCodeRequired
		}

		mfaSecret, err := h.repository.GetMfaTotpSecretValidationByUserID(ctx, user.ID)
		if err != nil {
			return nil, err
		}
		if mfaSecret == nil || !mfaSecret.IsValidated {
			return nil, &errors.ErrMfaUserSecretNotFound
		}

		valid := totp.Validate(*command.TOTPCode, mfaSecret.Secret)
		if !valid {
			failureDetails := "invalid TOTP code"
			auditLog := entities.NewAuditLog(user.ID, command.ApplicationID, constants.AuditEventFailedReauth, "", "", "failure", &failureDetails)
			_ = h.repository.AddAuditLog(ctx, auditLog)
			return nil, &errors.ErrInvalidMfaAuthAppCode
		}
	}

	// Revoke existing step-up tokens
	_ = h.repository.RevokeStepUpTokensByUserID(ctx, user.ID)

	// Create a new step-up token (valid for 5 minutes)
	stepUpToken := entities.NewStepUpToken(user.ID, command.ApplicationID)

	if err := h.repository.AddStepUpToken(ctx, stepUpToken); err != nil {
		return nil, err
	}

	// Log successful reauthentication
	auditLog := entities.NewAuditLog(user.ID, command.ApplicationID, constants.AuditEventReauthSuccess, "", "", "success", nil)
	_ = h.repository.AddAuditLog(ctx, auditLog)

	return &Response{
		StepUpToken: stepUpToken.Token,
		ExpiresIn:   300, // 5 minutes in seconds
	}, nil
}
