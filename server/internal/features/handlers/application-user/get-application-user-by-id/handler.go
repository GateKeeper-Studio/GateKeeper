package getapplicationuserbyid

import (
	"context"

	"github.com/gate-keeper/internal/domain/constants"
	"github.com/gate-keeper/internal/domain/errors"
	"github.com/gate-keeper/internal/infra/database/repositories"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
)

type Handler struct {
	repository IRepository
}

func New(q *pgstore.Queries) repositories.ServiceHandlerRs[Query, *Response] {
	return &Handler{
		repository: Repository{Store: q},
	}
}

func (s *Handler) Handler(ctx context.Context, query Query) (*Response, error) {
	user, err := s.repository.GetUserByID(ctx, query.UserID)

	if err != nil {
		return nil, nil
	}

	if user == nil {
		return nil, &errors.ErrUserNotFound
	}

	userProfile, err := s.repository.GetUserProfileByID(ctx, user.ID)

	if err != nil {
		return nil, nil
	}

	badges, err := s.repository.GetRolesByUserID(ctx, user.ID)

	if err != nil {
		return nil, nil
	}

	badgesResponse := make([]UserRoleResponse, 0)

	for _, badge := range badges {
		badgesResponse = append(badgesResponse, UserRoleResponse{
			ID:   badge.ID,
			Name: badge.Name,
		})
	}

	mfaMethods, err := s.repository.GetUserMfaMethods(ctx, user.ID)

	if err != nil {
		return nil, nil
	}

	isMfaEmailConfigured := false

	for _, method := range mfaMethods {
		if method.Type == constants.MfaMethodEmail && method.Enabled {
			isMfaEmailConfigured = true
			break
		}
	}

	isMfaAuthAppConfigured := false

	for _, method := range mfaMethods {
		if method.Type == constants.MfaMethodTotp && method.Enabled {
			isMfaAuthAppConfigured = true
			break
		}
	}

	return &Response{
		ID:                     user.ID,
		Email:                  user.Email,
		IsActive:               user.IsActive,
		DisplayName:            userProfile.DisplayName,
		FirstName:              userProfile.FirstName,
		Lastname:               userProfile.LastName,
		Address:                userProfile.Address,
		PhotoURL:               userProfile.PhotoURL,
		IsEmailVerified:        user.IsEmailConfirmed,
		Preferred2FAMethod:     user.Preferred2FAMethod,
		Badges:                 badgesResponse,
		IsMfaEmailConfigured:   isMfaEmailConfigured,
		IsMfaAuthAppConfigured: isMfaAuthAppConfigured,
	}, nil
}
