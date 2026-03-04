package accountrevokesession

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
	// 1. Verify session exists and belongs to user
	session, err := h.repository.GetUserSessionByID(ctx, command.SessionID, command.UserID)
	if err != nil {
		return nil, err
	}
	if session == nil {
		return nil, &errors.ErrSessionNotFound
	}

	// 2. Revoke the session
	if err := h.repository.RevokeUserSessionByID(ctx, command.SessionID, command.UserID); err != nil {
		return nil, err
	}

	// 3. Log security event
	details := "session_id: " + command.SessionID.String()
	auditLog := entities.NewAuditLog(command.UserID, session.ApplicationID,
		constants.AuditEventSessionRevoked, command.IPAddress, command.UserAgent, "success", &details)
	_ = h.repository.AddAuditLog(ctx, auditLog)

	return &Response{
		Message: "Session revoked successfully",
	}, nil
}
