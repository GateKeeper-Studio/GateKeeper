package repositories

import (
	"context"
	"strings"

	"github.com/gate-keeper/internal/domain/entities"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	"github.com/google/uuid"
	"github.com/jackc/pgx/v5/pgtype"
)

// IApplicationRepository defines all operations related to the Application entity.
type IApplicationRepository interface {
	GetApplicationByID(ctx context.Context, applicationID uuid.UUID) (*entities.Application, error)
	AddApplication(ctx context.Context, application *entities.Application) error
	UpdateApplication(ctx context.Context, application *entities.Application) error
	RemoveApplication(ctx context.Context, applicationID uuid.UUID) error
	CheckIfApplicationExists(ctx context.Context, applicationID uuid.UUID) (bool, error)
	ListApplicationsFromOrganization(ctx context.Context, organizationID uuid.UUID) (*[]entities.Application, error)
}

// ApplicationRepository is the shared implementation for Application-related DB operations.
type ApplicationRepository struct {
	Store *pgstore.Queries
}

func (r ApplicationRepository) GetApplicationByID(ctx context.Context, applicationID uuid.UUID) (*entities.Application, error) {
	application, err := r.Store.GetApplicationByID(ctx, applicationID)

	if err == ErrNoRows {
		return nil, nil
	}

	if err != nil {
		return nil, err
	}

	return &entities.Application{
		ID:                  application.ID,
		Name:                application.Name,
		Description:         application.Description,
		OrganizationID:      application.OrganizationID,
		CreatedAt:           application.CreatedAt.Time,
		IsActive:            application.IsActive,
		HasMfaAuthApp:       application.HasMfaAuthApp,
		HasMfaEmail:         application.HasMfaEmail,
		HasMfaWebauthn:      application.HasMfaWebauthn,
		PasswordHashSecret:  application.PasswordHashSecret,
		UpdatedAt:           application.UpdatedAt,
		Badges:              strings.Split(*application.Badges, ","),
		CanSelfSignUp:       application.CanSelfSignUp,
		CanSelfForgotPass:   application.CanSelfForgotPass,
		RefreshTokenTTLDays: int(application.RefreshTokenTtlDays),
	}, nil
}

func (r ApplicationRepository) AddApplication(ctx context.Context, newApplication *entities.Application) error {
	badges := strings.Join(newApplication.Badges, ",")

	return r.Store.AddApplication(ctx, pgstore.AddApplicationParams{
		ID:                 newApplication.ID,
		Name:               newApplication.Name,
		Description:        newApplication.Description,
		OrganizationID:     newApplication.OrganizationID,
		IsActive:           newApplication.IsActive,
		HasMfaAuthApp:      newApplication.HasMfaAuthApp,
		HasMfaEmail:        newApplication.HasMfaEmail,
		HasMfaWebauthn:     newApplication.HasMfaWebauthn,
		PasswordHashSecret: newApplication.PasswordHashSecret,
		Badges:             &badges,
		UpdatedAt:          newApplication.UpdatedAt,
		CanSelfSignUp:      newApplication.CanSelfSignUp,
		CanSelfForgotPass:  newApplication.CanSelfForgotPass,
		CreatedAt:          pgtype.Timestamp{Time: newApplication.CreatedAt, Valid: true},
	})
}

func (r ApplicationRepository) UpdateApplication(ctx context.Context, newApplication *entities.Application) error {
	badges := strings.Join(newApplication.Badges, ",")

	return r.Store.UpdateApplication(ctx, pgstore.UpdateApplicationParams{
		ID:                  newApplication.ID,
		Name:                newApplication.Name,
		Description:         newApplication.Description,
		HasMfaAuthApp:       newApplication.HasMfaAuthApp,
		Badges:              &badges,
		IsActive:            newApplication.IsActive,
		HasMfaEmail:         newApplication.HasMfaEmail,
		UpdatedAt:           newApplication.UpdatedAt,
		CanSelfSignUp:       newApplication.CanSelfSignUp,
		CanSelfForgotPass:   newApplication.CanSelfForgotPass,
		RefreshTokenTtlDays: int32(newApplication.RefreshTokenTTLDays),
	})
}

func (r ApplicationRepository) RemoveApplication(ctx context.Context, applicationID uuid.UUID) error {
	return r.Store.DeleteApplication(ctx, applicationID)
}

func (r ApplicationRepository) CheckIfApplicationExists(ctx context.Context, applicationID uuid.UUID) (bool, error) {
	exists, err := r.Store.CheckIfApplicationExists(ctx, applicationID)
	if err != nil {
		return false, err
	}
	return exists, nil
}

func (r ApplicationRepository) ListApplicationsFromOrganization(ctx context.Context, organizationID uuid.UUID) (*[]entities.Application, error) {
	applications, err := r.Store.ListApplicationsFromOrganization(ctx, organizationID)

	if err != nil && err != ErrNoRows {
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
