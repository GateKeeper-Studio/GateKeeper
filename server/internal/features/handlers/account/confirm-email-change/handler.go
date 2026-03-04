package accountconfirmemailchange

import (
	"context"
	"time"

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
	// 1. Fetch email change request by token
	req, err := h.repository.GetEmailChangeRequestByToken(ctx, command.Token)
	if err != nil {
		return nil, err
	}
	if req == nil {
		return nil, &errors.ErrEmailChangeNotFound
	}

	// 2. Check if the token has expired
	if req.ExpiresAt.Before(time.Now().UTC()) {
		return nil, &errors.ErrEmailChangeExpired
	}

	// 3. Check if the new email is still available (may have been taken since request)
	exists, err := h.repository.IsUserExistsByEmail(ctx, req.NewEmail, req.ApplicationID)
	if err != nil {
		return nil, err
	}
	if exists {
		return nil, &errors.ErrEmailAlreadyInUse
	}

	// 4. Fetch the user
	user, err := h.repository.GetUserByID(ctx, req.UserID)
	if err != nil {
		return nil, err
	}
	if user == nil {
		return nil, &errors.ErrUserNotFound
	}

	// 5. Update the user's email
	user.Email = req.NewEmail
	if _, err := h.repository.UpdateUser(ctx, user); err != nil {
		return nil, err
	}

	// 6. Mark the email change request as confirmed
	if err := h.repository.ConfirmEmailChangeRequest(ctx, req); err != nil {
		return nil, err
	}

	// 7. Log security event
	details := "new_email: " + req.NewEmail
	auditLog := entities.NewAuditLog(user.ID, req.ApplicationID,
		constants.AuditEventEmailChanged, command.IPAddress, command.UserAgent, "success", &details)
	_ = h.repository.AddAuditLog(ctx, auditLog)

	return &Response{
		Message:  "Email address has been changed successfully",
		NewEmail: req.NewEmail,
	}, nil
}
