package createtenantuser

import (
	"context"
	"time"

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

	isEmailExists, err := s.repository.IsUserExistsByEmail(ctx, request.Email, request.TenantID)

	if err != nil {
		return nil, err
	}

	if isEmailExists {
		return nil, &errors.ErrUserAlreadyExists
	}

	hashedPassword, err := application_utils.HashPassword(*request.TemporaryPasswordHash, application.PasswordHashSecret)

	if err != nil {
		return nil, err
	}

	tenantUser, err := entities.CreateTenantUser(
		request.Email,
		request.TenantID,
		true, // shouldChangePass
	)

	if err != nil {
		return nil, err
	}

	// tenantUser.IsMfaAuthAppEnabled = request.IsMfaAuthAppEnabled
	// tenantUser.IsMfaEmailEnabled = request.IsMfaEmailEnabled
	tenantUser.IsEmailConfirmed = request.IsEmailConfirmed

	tenantUserProfile := entities.NewUserProfile(
		tenantUser.ID,
		request.FirstName,
		request.LastName,
		request.DisplayName,
		nil,
		nil,
		nil,
	)

	userCredentials := entities.NewUserCredentials(
		tenantUser.ID,
		hashedPassword,
		true, // shouldChangePass on next login
	)

	if err = s.repository.AddUser(ctx, tenantUser); err != nil {
		return nil, err
	}

	if err = s.repository.AddUserProfile(ctx, tenantUserProfile); err != nil {
		return nil, err
	}

	if err := s.repository.AddUserCredentials(ctx, userCredentials); err != nil {
		return nil, err
	}

	for _, roleID := range request.Roles {
		userRole := entities.UserRole{
			UserID:    tenantUser.ID,
			CreatedAt: time.Now(),
			RoleID:    roleID,
		}

		err = s.repository.AddUserRole(ctx, &userRole)

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
