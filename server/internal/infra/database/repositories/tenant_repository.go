package repositories

import (
	"context"

	"github.com/gate-keeper/internal/domain/entities"
	pgstore "github.com/gate-keeper/internal/infra/database/sqlc"
	"github.com/google/uuid"
	"github.com/jackc/pgx/v5/pgtype"
)

// ITenantRepository defines all operations related to the Tenant entity.
type ITenantRepository interface {
	GetTenantByID(ctx context.Context, id uuid.UUID) (*entities.Tenant, error)
	AddTenant(ctx context.Context, tenant *entities.Tenant) error
	UpdateTenant(ctx context.Context, tenant *entities.Tenant) error
	RemoveTenant(ctx context.Context, tenantID uuid.UUID) error
	ListTenants(ctx context.Context) (*[]entities.Tenant, error)
}

// TenantRepository is the shared implementation for Tenant-related DB operations.
type TenantRepository struct {
	Store *pgstore.Queries
}

func (r TenantRepository) GetTenantByID(ctx context.Context, id uuid.UUID) (*entities.Tenant, error) {
	tenant, err := r.Store.GetTenantByID(ctx, id)

	if err != nil && err != ErrNoRows {
		return nil, err
	}

	return &entities.Tenant{
		ID:                 tenant.ID,
		Name:               tenant.Name,
		CreatedAt:          tenant.CreatedAt.Time,
		UpdatedAt:          tenant.UpdatedAt,
		Description:        tenant.Description,
		PasswordHashSecret: tenant.PasswordHashSecret,
	}, nil
}

func (r TenantRepository) AddTenant(ctx context.Context, tenant *entities.Tenant) error {
	return r.Store.AddTenant(ctx, pgstore.AddTenantParams{
		ID:                 tenant.ID,
		Name:               tenant.Name,
		Description:        tenant.Description,
		PasswordHashSecret: tenant.PasswordHashSecret,
		CreatedAt:          pgtype.Timestamp{Time: tenant.CreatedAt, Valid: true},
	})
}

func (r TenantRepository) UpdateTenant(ctx context.Context, tenant *entities.Tenant) error {
	return r.Store.UpdateTenant(ctx, pgstore.UpdateTenantParams{
		ID:                 tenant.ID,
		Name:               tenant.Name,
		Description:        tenant.Description,
		PasswordHashSecret: tenant.PasswordHashSecret,
		UpdatedAt:          tenant.UpdatedAt,
	})
}

func (r TenantRepository) RemoveTenant(ctx context.Context, tenantID uuid.UUID) error {
	return r.Store.RemoveTenant(ctx, tenantID)
}

func (r TenantRepository) ListTenants(ctx context.Context) (*[]entities.Tenant, error) {
	tenants, err := r.Store.ListTenants(ctx)

	if err != nil && err != ErrNoRows {
		return nil, err
	}

	var tenantList []entities.Tenant
	for _, tenant := range tenants {
		tenantList = append(tenantList, entities.Tenant{
			ID:                 tenant.ID,
			Name:               tenant.Name,
			CreatedAt:          tenant.CreatedAt.Time,
			UpdatedAt:          tenant.UpdatedAt,
			Description:        tenant.Description,
			PasswordHashSecret: tenant.PasswordHashSecret,
		})
	}

	return &tenantList, nil
}
