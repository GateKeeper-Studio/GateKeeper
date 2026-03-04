package accountupdatepreferredmfa

import (
	"context"

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

	// If setting a preferred method, verify that the MFA method exists and is enabled
	if command.PreferredMethod != nil {
		mfaMethod, err := h.repository.GetMfaMethodByUserID(ctx, user.ID, *command.PreferredMethod)
		if err != nil {
			return nil, err
		}
		if mfaMethod == nil {
			return nil, &errors.ErrMfaMethodNotFound
		}
		if !mfaMethod.Enabled {
			return nil, &errors.ErrMfaMethodNotEnabled
		}
	}

	user.Preferred2FAMethod = command.PreferredMethod
	if _, err := h.repository.UpdateUser(ctx, user); err != nil {
		return nil, err
	}

	return &Response{
		Message:         "Preferred MFA method updated successfully",
		PreferredMethod: command.PreferredMethod,
	}, nil
}
