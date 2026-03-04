package accountlistmfamethods

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

	mfaMethods, err := h.repository.GetUserMfaMethods(ctx, user.ID)
	if err != nil {
		return nil, err
	}

	methods := make([]MfaMethodItem, 0, len(mfaMethods))
	for _, m := range mfaMethods {
		methods = append(methods, MfaMethodItem{
			Type:    m.Type,
			Enabled: m.Enabled,
		})
	}

	return &Response{
		PreferredMethod: user.Preferred2FAMethod,
		Methods:         methods,
	}, nil
}
