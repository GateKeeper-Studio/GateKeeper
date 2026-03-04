package me

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

	profile, err := h.repository.GetUserProfileByID(ctx, user.ID)
	if err != nil {
		return nil, err
	}

	var firstName, lastName, displayName string
	var phoneNumber, address *string
	if profile != nil {
		firstName = profile.FirstName
		lastName = profile.LastName
		displayName = profile.DisplayName
		phoneNumber = profile.PhoneNumber
		address = profile.Address
	}

	hasMfa := user.Preferred2FAMethod != nil && *user.Preferred2FAMethod != ""

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
		UserID:      user.ID.String(),
		Email:       user.Email,
		FirstName:   firstName,
		LastName:    lastName,
		DisplayName: displayName,
		PhoneNumber: phoneNumber,
		Address:     address,
		HasMfa:      hasMfa,
		MfaMethod:   user.Preferred2FAMethod,
		MfaMethods:  methods,
	}, nil
}
