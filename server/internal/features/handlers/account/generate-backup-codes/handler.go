package accountgeneratebackupcodes

import (
	"context"

	"github.com/gate-keeper/internal/domain/constants"
	"github.com/gate-keeper/internal/domain/entities"
	"github.com/gate-keeper/internal/domain/errors"
	"github.com/gate-keeper/internal/infra/database/repositories"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
)

const backupCodeCount = 10

type Handler struct {
	repository IRepository
}

func New(q *pgstore.Queries) repositories.ServiceHandlerRs[Command, *Response] {
	return &Handler{
		repository: NewRepository(q),
	}
}

func (h *Handler) Handler(ctx context.Context, command Command) (*Response, error) {
	// Step-up authentication is enforced by middleware

	// 1. Verify user exists
	user, err := h.repository.GetUserByID(ctx, command.UserID)
	if err != nil {
		return nil, err
	}
	if user == nil {
		return nil, &errors.ErrUserNotFound
	}

	// 2. Invalidate all old backup codes
	if err := h.repository.DeleteBackupCodesByUserID(ctx, user.ID); err != nil {
		return nil, err
	}

	// 3. Generate new backup codes (store hashed, return plaintext once)
	backupCodes, plaintextCodes := entities.GenerateBackupCodes(user.ID, backupCodeCount)

	for _, code := range backupCodes {
		if err := h.repository.AddBackupCode(ctx, code); err != nil {
			return nil, err
		}
	}

	// 4. Log security event
	auditLog := entities.NewAuditLog(user.ID, command.ApplicationID,
		constants.AuditEventBackupCodesGenerated, command.IPAddress, command.UserAgent, "success", nil)
	_ = h.repository.AddAuditLog(ctx, auditLog)

	return &Response{
		Codes:   plaintextCodes,
		Message: "Backup codes generated successfully. Store them securely — they will not be shown again.",
	}, nil
}
