package getapplicationuserbyid

import (
	"context"
	"strings"

	"github.com/gate-keeper/internal/domain/entities"
	"github.com/gate-keeper/internal/infra/database/repositories"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	"github.com/google/uuid"
)

type IRepository interface {
	GetOrganizationByID(ctx context.Context, organizationID uuid.UUID) (*entities.Organization, error)
	ListApplicationsFromOrganization(ctx context.Context, organizationID uuid.UUID) (*[]entities.Application, error)
	GetUserByID(ctx context.Context, userID uuid.UUID) (*entities.ApplicationUser, error)
	GetUserProfileByID(ctx context.Context, userID uuid.UUID) (*entities.UserProfile, error)
	GetRolesByUserID(ctx context.Context, userID uuid.UUID) ([]entities.ApplicationRole, error)
	GetUserMfaMethods(ctx context.Context, userID uuid.UUID) ([]*entities.MfaMethod, error)
}

type Repository struct {
	Store *pgstore.Queries
}

func (r Repository) GetOrganizationByID(ctx context.Context, organizationID uuid.UUID) (*entities.Organization, error) {
	organization, err := r.Store.GetOrganizationByID(ctx, organizationID)

	if err == repositories.ErrNoRows {
		return nil, nil
	}

	if err != nil {
		return nil, err
	}

	return &entities.Organization{
		ID:          organization.ID,
		Name:        organization.Name,
		Description: organization.Description,
		CreatedAt:   organization.CreatedAt.Time,
		UpdatedAt:   organization.UpdatedAt,
	}, nil
}

func (r Repository) ListApplicationsFromOrganization(ctx context.Context, organizationID uuid.UUID) (*[]entities.Application, error) {
	applications, err := r.Store.ListApplicationsFromOrganization(ctx, organizationID)

	if err != nil && err != repositories.ErrNoRows {
		return nil, err
	}

	applicationList := make([]entities.Application, 0)

	for _, application := range applications {
		if application.Badges == nil {
			application.Badges = new(string)
		}

		applicationList = append(applicationList, entities.Application{
			ID:             application.ID,
			Name:           application.Name,
			Description:    application.Description,
			OrganizationID: application.OrganizationID,
			CreatedAt:      application.CreatedAt.Time,
			Badges:         strings.Split(*application.Badges, ","),
			UpdatedAt:      application.UpdatedAt,
		})
	}

	return &applicationList, nil
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

func (r Repository) GetUserMfaMethods(ctx context.Context, userID uuid.UUID) ([]*entities.MfaMethod, error) {
	mfaMethods, err := r.Store.GetUserMfaMethods(ctx, userID)

	if err != nil {
		return nil, err
	}

	var result []*entities.MfaMethod

	for _, method := range mfaMethods {
		result = append(result, &entities.MfaMethod{
			ID:         method.ID,
			Type:       method.Type,
			UserID:     method.UserID,
			Enabled:    method.Enabled,
			CreatedAt:  method.CreatedAt.Time,
			LastUsedAt: method.LastUsedAt,
		})
	}

	return result, nil
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

func (r Repository) GetUserProfileByID(ctx context.Context, userID uuid.UUID) (*entities.UserProfile, error) {
	userProfile, err := r.Store.GetUserProfileByUserId(ctx, userID)

	if err != nil {
		return nil, err
	}

	return &entities.UserProfile{
		UserID:      userProfile.UserID,
		DisplayName: userProfile.DisplayName,
		FirstName:   userProfile.FirstName,
		LastName:    userProfile.LastName,
		Address:     userProfile.Address,
		PhoneNumber: userProfile.PhoneNumber,
		PhotoURL:    userProfile.PhotoUrl,
	}, nil
}
