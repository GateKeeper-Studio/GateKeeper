package session

import (
	"context"

	application_utils "github.com/gate-keeper/internal/features/utils"
	"github.com/gate-keeper/internal/infra/database/repositories"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
)

type Handler struct {
	// repository IRepository
}

func New(q *pgstore.Queries) repositories.ServiceHandlerRs[Command, *Response] {
	return &Handler{
		// repository: NewRepository(q),
	}
}

func (s *Handler) Handler(ctx context.Context, command Command) (*Response, error) {
	token, err := application_utils.DecodeToken(command.AccessToken)

	if err != nil {
		return nil, err
	}

	return &Response{
		User: UserData{
			ID:            token.UserID,
			DisplayName:   token.DisplayName,
			FirstName:     token.FirstName,
			LastName:      token.LastName,
			Email:         token.Email,
			ApplicationID: token.ApplicationID,
			PhotoURL:      nil,
		},
		AccessToken: command.AccessToken,
	}, nil
}
