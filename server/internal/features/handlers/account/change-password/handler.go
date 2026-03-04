package accountchangepassword

import (
	"context"
	"regexp"
	"time"

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
		repository: NewRepository(q),
	}
}

// passwordStrengthRegex enforces: >=8 chars, at least 1 uppercase, 1 lowercase, 1 digit, 1 special char
var (
	hasUpperCase   = regexp.MustCompile(`[A-Z]`)
	hasLowerCase   = regexp.MustCompile(`[a-z]`)
	hasDigit       = regexp.MustCompile(`[0-9]`)
	hasSpecialChar = regexp.MustCompile(`[^a-zA-Z0-9]`)
)

func validatePasswordStrength(password string) bool {
	if len(password) < 8 {
		return false
	}
	return hasUpperCase.MatchString(password) &&
		hasLowerCase.MatchString(password) &&
		hasDigit.MatchString(password) &&
		hasSpecialChar.MatchString(password)
}

func (h *Handler) Handler(ctx context.Context, command Command) (*Response, error) {
	// 1. Fetch user
	user, err := h.repository.GetUserByID(ctx, command.UserID)
	if err != nil {
		return nil, err
	}
	if user == nil {
		return nil, &errors.ErrUserNotFound
	}

	// 2. Fetch credentials
	userCredentials, err := h.repository.GetUserCredentialsByUserID(ctx, user.ID)
	if err != nil {
		return nil, err
	}
	if userCredentials == nil {
		return nil, &errors.ErrUserCredentialsNotFound
	}

	// 3. Verify current password
	isPasswordCorrect, err := application_utils.ComparePassword(userCredentials.PasswordHash, command.CurrentPassword)
	if err != nil {
		return nil, err
	}
	if !isPasswordCorrect {
		// Log failed attempt
		auditLog := entities.NewAuditLog(user.ID, command.ApplicationID,
			constants.AuditEventFailedReauth, command.IPAddress, command.UserAgent, "failure", nil)
		_ = h.repository.AddAuditLog(ctx, auditLog)
		return nil, &errors.ErrCurrentPasswordIncorrect
	}

	// 4. Validate password strength
	if !validatePasswordStrength(command.NewPassword) {
		return nil, &errors.ErrPasswordTooWeak
	}

	// 5. Ensure new password is different from current
	isSamePassword, _ := application_utils.ComparePassword(userCredentials.PasswordHash, command.NewPassword)
	if isSamePassword {
		return nil, &errors.ErrPasswordSameAsCurrent
	}

	// 6. Fetch application for hashing secret
	application, err := h.repository.GetApplicationByID(ctx, command.ApplicationID)
	if err != nil {
		return nil, err
	}
	if application == nil {
		return nil, &errors.ErrApplicationNotFound
	}

	// 7. Hash new password using Argon2
	newHashedPassword, err := application_utils.HashPassword(command.NewPassword, application.PasswordHashSecret)
	if err != nil {
		return nil, err
	}

	// 8. Update credentials
	now := time.Now().UTC()
	userCredentials.PasswordHash = newHashedPassword
	userCredentials.ShouldChangePass = false
	userCredentials.UpdatedAt = &now

	if err := h.repository.UpdateUserCredentials(ctx, userCredentials); err != nil {
		return nil, err
	}

	// 9. Invalidate all existing sessions (security requirement)
	_ = h.repository.RevokeAllUserSessions(ctx, user.ID)

	// 10. Invalidate all refresh tokens
	_ = h.repository.RevokeRefreshTokenFromUser(ctx, user.ID)

	// 11. Log security event
	auditLog := entities.NewAuditLog(user.ID, command.ApplicationID,
		constants.AuditEventPasswordChanged, command.IPAddress, command.UserAgent, "success", nil)
	_ = h.repository.AddAuditLog(ctx, auditLog)

	return &Response{
		Message: "Password changed successfully. All sessions have been revoked.",
	}, nil
}
