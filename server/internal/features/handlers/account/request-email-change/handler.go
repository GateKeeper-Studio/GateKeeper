package accountrequestemailchange

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
	// Step-up authentication is enforced by middleware

	// 1. Verify user exists
	user, err := h.repository.GetUserByID(ctx, command.UserID)
	if err != nil {
		return nil, err
	}
	if user == nil {
		return nil, &errors.ErrUserNotFound
	}

	// 2. Check if new email is already in use
	exists, err := h.repository.IsUserExistsByEmail(ctx, command.NewEmail, command.ApplicationID)
	if err != nil {
		return nil, err
	}
	if exists {
		return nil, &errors.ErrEmailAlreadyInUse
	}

	// 3. Revoke any pending email change requests
	revokeReq := &entities.EmailChangeRequest{UserID: user.ID}
	_ = h.repository.RevokeEmailChangeRequestsByUserID(ctx, revokeReq)

	// 4. Create new email change request with signed, expiring token
	emailChangeReq := entities.NewEmailChangeRequest(user.ID, command.ApplicationID, command.NewEmail)
	if err := h.repository.AddEmailChangeRequest(ctx, emailChangeReq); err != nil {
		return nil, err
	}

	// 5. Log security event
	details := "new_email: " + command.NewEmail
	auditLog := entities.NewAuditLog(user.ID, command.ApplicationID,
		constants.AuditEventEmailChangeRequested, command.IPAddress, command.UserAgent, "success", &details)
	_ = h.repository.AddAuditLog(ctx, auditLog)

	// NOTE: In production, send a verification email to the new email address
	// containing the token. The email is NOT updated until confirmed.
	// mailService.SendEmailChangeVerification(command.NewEmail, emailChangeReq.Token)

	return &Response{
		Message: "Verification email sent to the new address. Please confirm to complete the change.",
	}, nil
}
