package createapplicationuser

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

	isEmailExists, err := s.repository.IsUserExistsByEmail(ctx, request.Email, request.ApplicationID)

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

	applicationUser, err := entities.CreateApplicationUser(
		request.Email,
		&hashedPassword,
		request.ApplicationID,
		true, // shouldChangePass
	)

	if err != nil {
		return nil, err
	}

	// applicationUser.IsMfaAuthAppEnabled = request.IsMfaAuthAppEnabled
	// applicationUser.IsMfaEmailEnabled = request.IsMfaEmailEnabled
	applicationUser.IsEmailConfirmed = request.IsEmailConfirmed

	applicationUserProfile := entities.NewUserProfile(
		applicationUser.ID,
		request.FirstName,
		request.LastName,
		request.DisplayName,
		nil,
		nil,
		nil,
	)

	userCredentials := entities.NewUserCredentials(
		applicationUser.ID,
		hashedPassword,
		true, // shouldChangePass on next login
	)

	if err = s.repository.AddUser(ctx, applicationUser); err != nil {
		return nil, err
	}

	if err = s.repository.AddUserProfile(ctx, applicationUserProfile); err != nil {
		return nil, err
	}

	if err := s.repository.AddUserCredentials(ctx, userCredentials); err != nil {
		return nil, err
	}

	for _, roleID := range request.Roles {
		userRole := entities.UserRole{
			UserID:    applicationUser.ID,
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
		ID:          applicationUser.ID,
		DisplayName: applicationUserProfile.DisplayName,
		Email:       applicationUser.Email,
		Roles:       roles,
	}, nil
}
