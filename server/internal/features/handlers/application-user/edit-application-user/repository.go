package editapplicationuser

import (
	"context"
	"strings"
	"time"

	"github.com/gate-keeper/internal/domain/entities"
	"github.com/gate-keeper/internal/infra/database/repositories"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	"github.com/google/uuid"
	"github.com/jackc/pgx/v5/pgtype"
)

type IRepository interface {
	GetApplicationByID(ctx context.Context, applicationID uuid.UUID) (*entities.Application, error)
	GetUserByID(ctx context.Context, userID uuid.UUID) (*entities.ApplicationUser, error)
	UpdateUser(ctx context.Context, user *entities.ApplicationUser) (*entities.ApplicationUser, error)
	EditUserProfile(ctx context.Context, updatedUser *entities.UserProfile) error
	GetRolesByUserID(ctx context.Context, userID uuid.UUID) ([]entities.ApplicationRole, error)
	RemoveUserRole(ctx context.Context, userRole *entities.UserRole) error
	AddUserRole(ctx context.Context, newUserRole *entities.UserRole) error
	ListRolesFromApplication(ctx context.Context, applicationID uuid.UUID) (*[]entities.ApplicationRole, error)
	UpdateUserCredentials(ctx context.Context, userCredentials *entities.UserCredentials) error
	GetUserCredentialsByUserID(ctx context.Context, userID uuid.UUID) (*entities.UserCredentials, error)
}

type Repository struct {
	Store *pgstore.Queries
}

func (r Repository) GetUserCredentialsByUserID(ctx context.Context, userID uuid.UUID) (*entities.UserCredentials, error) {
	userCredentials, err := r.Store.GetUserCredentialsByUserID(ctx, userID)

	if err == repositories.ErrNoRows {
		return nil, nil
	}

	if err != nil {
		return nil, err
	}

	return &entities.UserCredentials{
		ID:                userCredentials.ID,
		UserID:            userCredentials.UserID,
		PasswordAlgorithm: userCredentials.PasswordAlgorithm,
		PasswordHash:      userCredentials.PasswordHash,
		ShouldChangePass:  userCredentials.ShouldChangePass,
		CreatedAt:         userCredentials.CreatedAt.Time,
		UpdatedAt:         userCredentials.UpdatedAt,
	}, nil
}

func (r Repository) UpdateUserCredentials(ctx context.Context, userCredentials *entities.UserCredentials) error {
	err := r.Store.UpdateUserCredentials(ctx, pgstore.UpdateUserCredentialsParams{
		UserID:            userCredentials.UserID,
		PasswordHash:      userCredentials.PasswordHash,
		PasswordAlgorithm: userCredentials.PasswordAlgorithm,
		ShouldChangePass:  userCredentials.ShouldChangePass,
		UpdatedAt:         userCredentials.UpdatedAt,
	})

	if err != nil {
		return err
	}

	return nil
}

func (r Repository) ListRolesFromApplication(ctx context.Context, applicationID uuid.UUID) (*[]entities.ApplicationRole, error) {
	roles, err := r.Store.ListRolesFromApplication(ctx, applicationID)

	if err != nil && err != repositories.ErrNoRows {
		return nil, err
	}

	var applicationRoles []entities.ApplicationRole

	for _, role := range roles {
		applicationRoles = append(applicationRoles, entities.ApplicationRole{
			ID:            role.ID,
			ApplicationID: role.ApplicationID,
			Name:          role.Name,
			Description:   role.Description,
			CreatedAt:     role.CreatedAt.Time,
			UpdatedAt:     role.UpdatedAt,
		})
	}

	return &applicationRoles, nil
}

func (r Repository) AddUserRole(ctx context.Context, newUserRole *entities.UserRole) error {
	err := r.Store.AddUserRole(ctx, pgstore.AddUserRoleParams{
		UserID:    newUserRole.UserID,
		RoleID:    newUserRole.RoleID,
		CreatedAt: pgtype.Timestamp{Time: newUserRole.CreatedAt, Valid: true},
	})

	return err
}

func (r Repository) RemoveUserRole(ctx context.Context, userRole *entities.UserRole) error {
	err := r.Store.RemoveUserRole(ctx, pgstore.RemoveUserRoleParams{
		UserID: userRole.UserID,
		RoleID: userRole.RoleID,
	})

	return err
}

func (r Repository) GetRolesByUserID(ctx context.Context, userID uuid.UUID) ([]entities.ApplicationRole, error) {
	roles, err := r.Store.GetUserRoles(ctx, userID)

	if err != nil {
		return nil, err
	}

	var applicationRoles []entities.ApplicationRole

	for _, role := range roles {
		applicationRoles = append(applicationRoles, entities.ApplicationRole{
			ID:   role.ID,
			Name: role.Name,
		})
	}

	return applicationRoles, nil
}

func (r Repository) EditUserProfile(ctx context.Context, newUserProfile *entities.UserProfile) error {
	err := r.Store.UpdateUserProfile(ctx, pgstore.UpdateUserProfileParams{
		UserID:      newUserProfile.UserID,
		DisplayName: newUserProfile.DisplayName,
		FirstName:   newUserProfile.FirstName,
		LastName:    newUserProfile.LastName,
		Address:     newUserProfile.Address,
		PhoneNumber: newUserProfile.PhoneNumber,
		PhotoUrl:    newUserProfile.PhotoURL,
	})

	return err
}

func (r Repository) UpdateUser(ctx context.Context, user *entities.ApplicationUser) (*entities.ApplicationUser, error) {
	now := time.Now().UTC()

	err := r.Store.UpdateUser(ctx, pgstore.UpdateUserParams{
		ID:                 user.ID,
		Email:              user.Email,
		UpdatedAt:          &now,
		IsActive:           user.IsActive,
		IsEmailConfirmed:   user.IsEmailConfirmed,
		Preferred2faMethod: user.Preferred2FAMethod,
	})

	return user, err
}

func (r Repository) GetApplicationByID(ctx context.Context, applicationID uuid.UUID) (*entities.Application, error) {
	application, err := r.Store.GetApplicationByID(ctx, applicationID)

	if err == repositories.ErrNoRows {
		return nil, nil
	}

	if err != nil {
		return nil, err
	}

	return &entities.Application{
		ID:                 application.ID,
		Name:               application.Name,
		Description:        application.Description,
		OrganizationID:     application.OrganizationID,
		CreatedAt:          application.CreatedAt.Time,
		IsActive:           application.IsActive,
		HasMfaAuthApp:      application.HasMfaAuthApp,
		HasMfaEmail:        application.HasMfaEmail,
		PasswordHashSecret: application.PasswordHashSecret,
		UpdatedAt:          application.UpdatedAt,
		Badges:             strings.Split(*application.Badges, ","),
		CanSelfSignUp:      application.CanSelfSignUp,
		CanSelfForgotPass:  application.CanSelfForgotPass,
	}, nil
}

func (r Repository) GetUserByID(ctx context.Context, id uuid.UUID) (*entities.ApplicationUser, error) {
	user, err := r.Store.GetUserById(ctx, id)

	if err != nil {
		return nil, err
	}

	return &entities.ApplicationUser{
		ID:                 user.ID,
		Email:              user.Email,
		CreatedAt:          user.CreatedAt.Time,
		UpdatedAt:          user.UpdatedAt,
		IsActive:           user.IsActive,
		IsEmailConfirmed:   user.IsEmailConfirmed,
		ApplicationID:      user.ApplicationID,
		Preferred2FAMethod: user.Preferred2faMethod,
	}, nil
}
