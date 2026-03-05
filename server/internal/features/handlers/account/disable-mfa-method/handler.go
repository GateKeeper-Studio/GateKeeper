package accountdisablemfamethod

import (
	"context"

	"github.com/gate-keeper/internal/domain/constants"
	"github.com/gate-keeper/internal/domain/entities"
	"github.com/gate-keeper/internal/domain/errors"
	"github.com/gate-keeper/internal/infra/database/repositories"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
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

	mfaMethod, err := h.repository.GetMfaMethodByUserID(ctx, user.ID, command.Method)
	if err != nil {
		return nil, err
	}
	if mfaMethod == nil || !mfaMethod.Enabled {
		return nil, &errors.ErrMfaNotEnabled
	}

	// Disable the MFA method
	if err := h.repository.DisableMfaMethod(ctx, mfaMethod.ID); err != nil {
		return nil, err
	}

	// For TOTP, also revoke secrets
	if command.Method == constants.MfaMethodTotp {
		if err := h.repository.RevokeTotpSecretsByUserID(ctx, user.ID); err != nil {
			return nil, err
		}
	}

	// If the disabled method was the preferred one, clear or update preferred method
	if user.Preferred2FAMethod != nil && *user.Preferred2FAMethod == command.Method {
		// Try to find another enabled method to fall back to
		remainingMethods, err := h.repository.GetUserMfaMethods(ctx, user.ID)
		if err != nil {
			return nil, err
		}

		var fallbackMethod *string
		for _, m := range remainingMethods {
			if m.ID != mfaMethod.ID && m.Enabled {
				fallbackMethod = &m.Type
				break
			}
		}

		user.Preferred2FAMethod = fallbackMethod
		if _, err := h.repository.UpdateUser(ctx, user); err != nil {
			return nil, err
		}
	}

	// If no MFA methods remain enabled, delete backup codes
	remainingMethods, err := h.repository.GetUserMfaMethods(ctx, user.ID)
	if err != nil {
		return nil, err
	}
	hasEnabledMethod := false
	for _, m := range remainingMethods {
		if m.ID != mfaMethod.ID && m.Enabled {
			hasEnabledMethod = true
			break
		}
	}
	if !hasEnabledMethod {
		_ = h.repository.DeleteBackupCodesByUserID(ctx, user.ID)
	}

	// Log audit event
	auditLog := entities.NewAuditLog(user.ID, user.TenantID,
		constants.AuditEventMfaDisabled, command.IPAddress, command.UserAgent, "success", nil)
	_ = h.repository.AddAuditLog(ctx, auditLog)

	return &Response{
		Message: "MFA method disabled successfully",
	}, nil
}
