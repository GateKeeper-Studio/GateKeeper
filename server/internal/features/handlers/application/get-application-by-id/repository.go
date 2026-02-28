package getapplicationbyid

import (
	"context"
	"encoding/json"
	"strings"

	"github.com/gate-keeper/internal/domain/entities"
	"github.com/gate-keeper/internal/infra/database/repositories"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	"github.com/google/uuid"
)

type ApplicationUsersData struct {
	TotalCount int                `json:"totalCount"`
	Data       []ApplicationUsers `json:"data"`
}

type ApplicationRolesData struct {
	TotalCount int                `json:"totalCount"`
	Data       []ApplicationRoles `json:"data"`
}

type ApplicationUsers struct {
	ID          uuid.UUID          `json:"id"`
	DisplayName string             `json:"displayName"`
	Email       string             `json:"email"`
	Roles       []ApplicationRoles `json:"roles"`
}

type ApplicationRoles struct {
	ID          uuid.UUID `json:"id"`
	Name        string    `json:"name"`
	Description *string   `json:"description"`
}

type IRepository interface {
	GetApplicationByID(ctx context.Context, applicationID uuid.UUID) (*entities.Application, error)
	GetUsersByApplicationID(ctx context.Context, applicationID uuid.UUID, limit, offset int) (*ApplicationUsersData, error)
	ListSecretsFromApplication(ctx context.Context, applicationID uuid.UUID) (*[]entities.ApplicationSecret, error)
	ListRolesFromApplication(ctx context.Context, applicationID uuid.UUID) (*[]entities.ApplicationRole, error)
	GetApplicationOAuthProvidersByApplicationID(ctx context.Context, applicationID uuid.UUID) (*[]ApplicationProviders, error)
}

type Repository struct {
	Store *pgstore.Queries
}

func (r Repository) GetApplicationOAuthProvidersByApplicationID(ctx context.Context, applicationID uuid.UUID) (*[]ApplicationProviders, error) {
	providers, err := r.Store.GetApplicationOauthProvidersByApplicationID(ctx, applicationID)

	if err != nil && err != repositories.ErrNoRows {
		return nil, err
	}

	var applicationProviders []ApplicationProviders
	for _, provider := range providers {
		applicationProviders = append(applicationProviders, ApplicationProviders{
			ID:           provider.ID,
			Name:         provider.Name,
			ClientID:     provider.ClientID,
			ClientSecret: provider.ClientSecret,
			RedirectURI:  provider.RedirectUri,
			UpdatedAt:    provider.UpdatedAt,
			IsEnabled:    provider.Enabled,
			CreatedAt:    provider.CreatedAt.Time,
		})
	}
	return &applicationProviders, nil
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
		HasMfaWebauthn:     application.HasMfaWebauthn,
		PasswordHashSecret: application.PasswordHashSecret,
		UpdatedAt:          application.UpdatedAt,
		Badges:             strings.Split(*application.Badges, ","),
		CanSelfSignUp:      application.CanSelfSignUp,
		CanSelfForgotPass:  application.CanSelfForgotPass,
	}, nil
}

func (r Repository) GetUsersByApplicationID(ctx context.Context, applicationID uuid.UUID, limit, offset int) (*ApplicationUsersData, error) {
	users, err := r.Store.GetUsersByApplicationID(ctx, pgstore.GetUsersByApplicationIDParams{
		ApplicationID: applicationID,
		Limit:         int32(limit),
		Offset:        int32(offset),
	})

	if err != nil {
		return nil, err
	}

	totalUsers := 0

	if len(users) > 0 {
		totalUsers = int(users[0].TotalUsers)
	}

	result := ApplicationUsersData{
		TotalCount: totalUsers,
		Data:       []ApplicationUsers{},
	}

	for _, user := range users {
		roles := []ApplicationRoles{}

		err := json.Unmarshal(user.Roles, &roles)

		if err != nil {
			return nil, err
		}

		result.Data = append(result.Data, ApplicationUsers{
			ID:          user.ID,
			DisplayName: *user.DisplayName,
			Email:       user.Email,
			Roles:       roles,
		})
	}

	return &result, nil
}

func (r Repository) ListSecretsFromApplication(ctx context.Context, applicationID uuid.UUID) (*[]entities.ApplicationSecret, error) {
	secrets, err := r.Store.ListSecretsFromApplication(ctx, applicationID)

	if err != nil && err != repositories.ErrNoRows {
		return nil, err
	}

	var applicationSecrets []entities.ApplicationSecret

	for _, secret := range secrets {
		applicationSecrets = append(applicationSecrets, entities.ApplicationSecret{
			ID:            secret.ID,
			ApplicationID: secret.ApplicationID,
			Name:          secret.Name,
			Value:         secret.Value,
			CreatedAt:     secret.CreatedAt.Time,
			UpdatedAt:     secret.UpdatedAt,
			ExpiresAt:     secret.ExpiresAt,
		})
	}

	return &applicationSecrets, nil
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
