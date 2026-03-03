package repositories

import (
	"context"

	"github.com/gate-keeper/internal/domain/entities"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	"github.com/google/uuid"
	"github.com/jackc/pgx/v5/pgtype"
)

// IOrganizationRepository defines all operations related to the Organization entity.
type IOrganizationRepository interface {
	GetOrganizationByID(ctx context.Context, id uuid.UUID) (*entities.Organization, error)
	AddOrganization(ctx context.Context, organization *entities.Organization) error
	UpdateOrganization(ctx context.Context, organization *entities.Organization) error
	RemoveOrganization(ctx context.Context, organizationID uuid.UUID) error
	ListOrganizations(ctx context.Context) (*[]entities.Organization, error)
}

// OrganizationRepository is the shared implementation for Organization-related DB operations.
type OrganizationRepository struct {
	Store *pgstore.Queries
}

func (r OrganizationRepository) GetOrganizationByID(ctx context.Context, id uuid.UUID) (*entities.Organization, error) {
	organization, err := r.Store.GetOrganizationByID(ctx, id)

	if err != nil && err != ErrNoRows {
		return nil, err
	}

	return &entities.Organization{
		ID:          organization.ID,
		Name:        organization.Name,
		CreatedAt:   organization.CreatedAt.Time,
		UpdatedAt:   organization.UpdatedAt,
		Description: organization.Description,
	}, nil
}

func (r OrganizationRepository) AddOrganization(ctx context.Context, organization *entities.Organization) error {
	return r.Store.AddOrganization(ctx, pgstore.AddOrganizationParams{
		UserID:      organization.ID,
		Name:        organization.Name,
		Description: organization.Description,
		CreatedAt:   pgtype.Timestamp{Time: organization.CreatedAt, Valid: true},
	})
}

func (r OrganizationRepository) UpdateOrganization(ctx context.Context, organization *entities.Organization) error {
	return r.Store.UpdateOrganization(ctx, pgstore.UpdateOrganizationParams{
		ID:          organization.ID,
		Name:        organization.Name,
		Description: organization.Description,
		UpdatedAt:   organization.UpdatedAt,
	})
}

func (r OrganizationRepository) RemoveOrganization(ctx context.Context, organizationID uuid.UUID) error {
	return r.Store.RemoveOrganization(ctx, organizationID)
}

func (r OrganizationRepository) ListOrganizations(ctx context.Context) (*[]entities.Organization, error) {
	organizations, err := r.Store.ListOrganizations(ctx)

	if err != nil && err != ErrNoRows {
		return nil, err
	}

	var organizationList []entities.Organization
	for _, organization := range organizations {
		organizationList = append(organizationList, entities.Organization{
			ID:          organization.ID,
			Name:        organization.Name,
			CreatedAt:   organization.CreatedAt.Time,
			UpdatedAt:   organization.UpdatedAt,
			Description: organization.Description,
		})
	}

	return &organizationList, nil
}
