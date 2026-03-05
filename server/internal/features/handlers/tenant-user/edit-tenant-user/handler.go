package edittenantuser

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
		repository: NewRepository(q),
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

	tenantUser, err := s.repository.GetUserByID(ctx, request.UserID)

	if err != nil {
		return nil, err
	}

	if tenantUser == nil {
		return nil, &errors.ErrUserNotFound
	}

	if request.TemporaryPasswordHash != nil {
		hashedPassword, err := application_utils.HashPassword(*request.TemporaryPasswordHash, application.PasswordHashSecret)

		if err != nil {
			return nil, err
		}

		userCredentials, err := s.repository.GetUserCredentialsByUserID(ctx, tenantUser.ID)

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

	tenantUser.UpdatedAt = &currentTime
	tenantUser.IsEmailConfirmed = request.IsEmailConfirmed
	tenantUser.IsActive = request.IsActive
	tenantUser.Preferred2FAMethod = request.Preferred2FAMethod

	tenantUserProfile := entities.NewUserProfile(
		tenantUser.ID,
		request.FirstName,
		request.LastName,
		request.DisplayName,
		nil,
		nil,
		nil,
	)

	if _, err = s.repository.UpdateUser(ctx, tenantUser); err != nil {
		return nil, err
	}

	if err = s.repository.EditUserProfile(ctx, tenantUserProfile); err != nil {
		return nil, err
	}

	userRoles, err := s.repository.GetRolesByUserID(ctx, tenantUser.ID)

	if err != nil {
		return nil, err
	}

	for _, role := range userRoles {
		userRole := entities.UserRole{
			UserID: tenantUser.ID,
			RoleID: role.ID,
		}

		_ = s.repository.RemoveUserRole(ctx, &userRole)
	}

	for _, roleID := range request.Roles {
		userRole := entities.UserRole{
			UserID:    tenantUser.ID,
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
		ID:          tenantUser.ID,
		DisplayName: tenantUserProfile.DisplayName,
		Email:       tenantUser.Email,
		Roles:       roles,
	}, nil
}
