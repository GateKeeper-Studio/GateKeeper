package accountlistsessions

import (
	"context"

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
	sessions, err := h.repository.GetActiveUserSessions(ctx, command.UserID)
	if err != nil {
		return nil, err
	}

	items := make([]SessionItem, 0, len(sessions))
	for _, s := range sessions {
		items = append(items, SessionItem{
			ID:           s.ID,
			IPAddress:    s.IPAddress,
			UserAgent:    s.UserAgent,
			Location:     s.Location,
			CreatedAt:    s.CreatedAt,
			LastActiveAt: s.LastActiveAt,
			ExpiresAt:    s.ExpiresAt,
		})
	}

	return &Response{
		Sessions: items,
		Total:    len(items),
	}, nil
}
