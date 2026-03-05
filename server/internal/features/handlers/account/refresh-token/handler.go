package accountrefreshtoken

import (
	"context"

	"github.com/gate-keeper/internal/domain/errors"
	application_utils "github.com/gate-keeper/internal/features/utils"
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
	// 1. Fetch the user to get current data
	user, err := h.repository.GetUserByID(ctx, command.UserID)
	if err != nil {
		return nil, err
	}
	if user == nil {
		return nil, &errors.ErrUserNotFound
	}

	// 2. Fetch user profile for name claims
	profile, err := h.repository.GetUserProfileByID(ctx, user.ID)
	if err != nil {
		return nil, err
	}

	var firstName, lastName, displayName string
	if profile != nil {
		firstName = profile.FirstName
		lastName = profile.LastName
		displayName = profile.DisplayName
	}

	// 3. Issue a fresh access token
	claims := application_utils.JWTClaims{
		UserID:      user.ID,
		FirstName:   firstName,
		LastName:    lastName,
		DisplayName: displayName,
		Email:       user.Email,
		TenantID:    user.TenantID,
	}

	accessToken, err := application_utils.CreateToken(claims)
	if err != nil {
		return nil, err
	}

	return &Response{
		AccessToken: accessToken,
		ExpiresIn:   15 * 60, // 15 minutes in seconds
	}, nil
}
