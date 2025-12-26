package createapplicationuser

import (
	"context"
	"strings"

	"github.com/gate-keeper/internal/domain/entities"
	"github.com/gate-keeper/internal/infra/database/repositories"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	"github.com/google/uuid"
	"github.com/jackc/pgx/v5/pgtype"
)

type IRepository interface {
	GetApplicationByID(ctx context.Context, applicationID uuid.UUID) (*entities.Application, error)
	IsUserExistsByEmail(ctx context.Context, email string, applicationID uuid.UUID) (bool, error)
	AddUser(ctx context.Context, user *entities.ApplicationUser) error
	AddUserProfile(ctx context.Context, userProfile *entities.UserProfile) error
	AddUserRole(ctx context.Context, userRole *entities.UserRole) error
	ListRolesFromApplication(ctx context.Context, applicationID uuid.UUID) (*[]entities.ApplicationRole, error)
	AddUserCredentials(ctx context.Context, userCredentials *entities.UserCredentials) error
}

type Repository struct {
	Store *pgstore.Queries
}

func (r Repository) AddUserCredentials(ctx context.Context, userCredentials *entities.UserCredentials) error {
	err := r.Store.AddUserCredentials(ctx, pgstore.AddUserCredentialsParams{
		ID:                userCredentials.ID,
		UserID:            userCredentials.UserID,
		PasswordAlgorithm: userCredentials.PasswordAlgorithm,
		PasswordHash:      userCredentials.PasswordHash,
		ShouldChangePass:  userCredentials.ShouldChangePass,
		CreatedAt:         pgtype.Timestamp{Time: userCredentials.CreatedAt, Valid: true},
		UpdatedAt:         userCredentials.UpdatedAt,
	})

	return err
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

func (r Repository) IsUserExistsByEmail(ctx context.Context, email string, applicationID uuid.UUID) (bool, error) {
	_, err := r.Store.GetUserByEmail(ctx, pgstore.GetUserByEmailParams{
		Email:         email,
		ApplicationID: applicationID,
	})

	if err != nil {
		return false, nil
	}

	return true, nil
}

func (r Repository) AddUser(ctx context.Context, newUser *entities.ApplicationUser) error {
	err := r.Store.AddUser(ctx, pgstore.AddUserParams{
		ID:               newUser.ID,
		Email:            newUser.Email,
		ApplicationID:    newUser.ApplicationID,
		CreatedAt:        pgtype.Timestamp{Time: newUser.CreatedAt, Valid: true},
		UpdatedAt:        newUser.UpdatedAt,
		IsActive:         newUser.IsActive,
		IsEmailConfirmed: newUser.IsEmailConfirmed,
	})

	return err
}

func (r Repository) AddUserProfile(ctx context.Context, newUserProfile *entities.UserProfile) error {
	err := r.Store.AddUserProfile(ctx, pgstore.AddUserProfileParams{
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
