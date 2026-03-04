package accountenableemailmfa

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

	mfaMethod, err := h.repository.GetMfaMethodByUserID(ctx, user.ID, constants.MfaMethodEmail)
	if err != nil {
		return nil, err
	}

	if mfaMethod != nil {
		// Method already exists — enable it if disabled
		if !mfaMethod.Enabled {
			if err := h.repository.EnableMfaMethod(ctx, mfaMethod.ID); err != nil {
				return nil, err
			}
		}

		return &Response{Message: "Email MFA enabled"}, nil
	}

	// Create a new email MFA method
	newMethod := entities.AddMfaMethod(user.ID, constants.MfaMethodEmail)
	if err := h.repository.AddMfaMethod(ctx, newMethod); err != nil {
		return nil, err
	}

	return &Response{Message: "Email MFA enabled"}, nil
}
