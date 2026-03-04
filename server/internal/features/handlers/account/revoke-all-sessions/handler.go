package accountrevokeallsessions

import (
	"context"

	"github.com/gate-keeper/internal/domain/constants"
	"github.com/gate-keeper/internal/domain/entities"
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

	// 1. Revoke all active sessions
	if err := h.repository.RevokeAllUserSessions(ctx, command.UserID); err != nil {
		return nil, err
	}

	// 2. Revoke all refresh tokens
	if err := h.repository.RevokeRefreshTokenFromUser(ctx, command.UserID); err != nil {
		return nil, err
	}

	// 3. Log security event
	auditLog := entities.NewAuditLog(command.UserID, command.ApplicationID,
		constants.AuditEventSessionsRevoked, command.IPAddress, command.UserAgent, "success", nil)
	_ = h.repository.AddAuditLog(ctx, auditLog)

	return &Response{
		Message: "All sessions have been revoked successfully",
	}, nil
}
