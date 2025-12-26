package editapplicationuser

import (
	"context"
	"time"

	"github.com/gate-keeper/internal/domain/constants"
	"github.com/gate-keeper/internal/domain/entities"
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
		repository: Repository{Store: q},
	}
}

func (s *Handler) Handler(ctx context.Context, request Command) (*Response, error) {
	application, err := s.repository.GetApplicationByID(ctx, request.ApplicationID)

	if err != nil {
		return nil, err
	}

	if application == nil {
		return nil, &errors.ErrApplicationNotFound
	}

	applicationUser, err := s.repository.GetUserByID(ctx, request.UserID)

	if err != nil {
		return nil, err
	}

	if applicationUser == nil {
		return nil, &errors.ErrUserNotFound
	}

	if request.TemporaryPasswordHash != nil {
		hashedPassword, err := application_utils.HashPassword(*request.TemporaryPasswordHash, application.PasswordHashSecret)

		if err != nil {
			return nil, err
		}

		userCredentials, err := s.repository.GetUserCredentialsByUserID(ctx, applicationUser.ID)

		if err != nil {
			return nil, err
		}

		userCredentials.PasswordHash = hashedPassword
		userCredentials.ShouldChangePass = true

		if err = s.repository.UpdateUserCredentials(ctx, userCredentials); err != nil {
			return nil, err
		}
	}

	if request.Preferred2FAMethod != nil &&
		*request.Preferred2FAMethod != constants.MfaMethodEmail &&
		*request.Preferred2FAMethod != constants.MfaMethodTotp &&
		*request.Preferred2FAMethod != constants.MfaMethodSms {
		return nil, &errors.ErrInvalid2FAMethod
	}

	currentTime := time.Now().UTC()

	applicationUser.UpdatedAt = &currentTime
	applicationUser.IsEmailConfirmed = request.IsEmailConfirmed
	applicationUser.IsActive = request.IsActive
	applicationUser.Preferred2FAMethod = request.Preferred2FAMethod

	applicationUserProfile := entities.NewUserProfile(
		applicationUser.ID,
		request.FirstName,
		request.LastName,
		request.DisplayName,
		nil,
		nil,
		nil,
	)

	if _, err = s.repository.UpdateUser(ctx, applicationUser); err != nil {
		return nil, err
	}

	if err = s.repository.EditUserProfile(ctx, applicationUserProfile); err != nil {
		return nil, err
	}

	userRoles, err := s.repository.GetRolesByUserID(ctx, applicationUser.ID)

	if err != nil {
		return nil, err
	}

	for _, role := range userRoles {
		userRole := entities.UserRole{
			UserID: applicationUser.ID,
			RoleID: role.ID,
		}

		_ = s.repository.RemoveUserRole(ctx, &userRole)
	}

	for _, roleID := range request.Roles {
		userRole := entities.UserRole{
			UserID:    applicationUser.ID,
			CreatedAt: time.Now(),
			RoleID:    roleID,
		}

		_ = s.repository.AddUserRole(ctx, &userRole)
	}

	roles := make([]applicationRoles, len(request.Roles))
	applicationRolesList, err := s.repository.ListRolesFromApplication(ctx, request.ApplicationID)

	if err != nil {
		return nil, err
	}

	for i, roleID := range request.Roles {
		for _, appRole := range *applicationRolesList {
			if appRole.ID == roleID {
				roles[i] = applicationRoles{
					ID:          appRole.ID,
					Name:        appRole.Name,
					Description: appRole.Description,
				}
			}
		}
	}

	return &Response{
		ID:          applicationUser.ID,
		DisplayName: applicationUserProfile.DisplayName,
		Email:       applicationUser.Email,
		Roles:       roles,
	}, nil
}
