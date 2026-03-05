package entities

import (
	"time"

	"github.com/google/uuid"
)

// TenantUser represents a user within a specific tenant.
// It's the core entity for user management, and each user is tied to a single tenant.
type TenantUser struct {
	ID                 uuid.UUID
	TenantID           uuid.UUID // references Tenant.ID, the tenant this user belongs to
	Email              string    // unique email for the user within the tenant
	CreatedAt          time.Time
	UpdatedAt          *time.Time
	IsActive           bool    // indicates if the user account is active
	IsEmailConfirmed   bool    // indicates if the user's email has been confirmed
	Preferred2FAMethod *string // e.g., "auth_app", "email", nil if 2FA is not set up
	// PasswordHash     *string   // hashed password, nil if using only external OAuth providers
	// ShouldChangePass bool // indicates if the user should change their password on next login
	// IsMfaAuthAppEnabled bool
	// IsMfaEmailEnabled   bool
	// TwoFactorSecret     *string
}

func CreateTenantUser(email string, tenantID uuid.UUID, shouldChangePass bool) (*TenantUser, error) {
	userId, err := uuid.NewV7()

	if err != nil {
		return nil, err
	}

	return &TenantUser{
		ID:                 userId,
		TenantID:           tenantID,
		Email:              email,
		CreatedAt:          time.Now().UTC(),
		UpdatedAt:          nil,
		IsActive:           true,
		IsEmailConfirmed:   false,
		Preferred2FAMethod: nil,
		// PasswordHash:     passwordHash,
		// ShouldChangePass: shouldChangePass,
		// IsMfaAuthAppEnabled: false,
		// IsMfaEmailEnabled:   false,
		// TwoFactorSecret:     nil,
	}, nil
}

func NewTenantUser(tenantID, id uuid.UUID, email string, createdAt time.Time, updatedAt *time.Time, isActive, isEmailConfirmed, IsMfaEmailEnabled, IsMfaAuthAppEnabled bool, twoFactorSecret *string, shouldChangePass bool) *TenantUser {
	return &TenantUser{
		ID:               id,
		TenantID:         tenantID,
		Email:            email,
		CreatedAt:        createdAt,
		UpdatedAt:        updatedAt,
		IsActive:         isActive,
		IsEmailConfirmed: isEmailConfirmed,
		// IsMfaAuthAppEnabled: IsMfaAuthAppEnabled,
		// IsMfaEmailEnabled:   IsMfaEmailEnabled,
		// TwoFactorSecret:     twoFactorSecret,
		// ShouldChangePass: shouldChangePass,
	}
}
