package accountupdateprofile

import (
	"context"

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

	profile := entities.NewUserProfile(
		user.ID,
		command.FirstName,
		command.LastName,
		command.DisplayName,
		command.PhoneNumber,
		command.Address,
		nil, // photoURL — not editable through this endpoint
	)

	// Check if profile already exists
	existingProfile, err := h.repository.GetUserProfileByID(ctx, user.ID)
	if err != nil {
		return nil, err
	}

	// Preserve existing photoURL if present
	if existingProfile != nil && existingProfile.PhotoURL != nil {
		profile.PhotoURL = existingProfile.PhotoURL
	}

	if err := h.repository.EditUserProfile(ctx, profile); err != nil {
		return nil, err
	}

	return &Response{
		FirstName:   profile.FirstName,
		LastName:    profile.LastName,
		DisplayName: profile.DisplayName,
		PhoneNumber: profile.PhoneNumber,
		Address:     profile.Address,
	}, nil
}
