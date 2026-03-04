package me

import (
	"context"
	"testing"

	"github.com/gate-keeper/internal/domain/entities"
	"github.com/google/uuid"
)

// ─── Mock Repository ──────────────────────────────────────────────────────
type mockRepository struct {
	user       *entities.ApplicationUser
	profile    *entities.UserProfile
	mfaMethods []*entities.MfaMethod
	userErr    error
	profErr    error
	mfaErr     error
}

func (m *mockRepository) GetUserByID(_ context.Context, _ uuid.UUID) (*entities.ApplicationUser, error) {
	return m.user, m.userErr
}

func (m *mockRepository) GetUserProfileByID(_ context.Context, _ uuid.UUID) (*entities.UserProfile, error) {
	return m.profile, m.profErr
}

func (m *mockRepository) GetUserMfaMethods(_ context.Context, _ uuid.UUID) ([]*entities.MfaMethod, error) {
	return m.mfaMethods, m.mfaErr
}

// ─── Tests ─────────────────────────────────────────────────────────────────

func TestHandler_ReturnsProfileWithoutMfa(t *testing.T) {
	userID := uuid.New()
	appID := uuid.New()

	h := &Handler{
		repository: &mockRepository{
			user: &entities.ApplicationUser{
				ID:                 userID,
				ApplicationID:      appID,
				Email:              "user@example.com",
				IsActive:           true,
				Preferred2FAMethod: nil,
			},
			profile: &entities.UserProfile{
				UserID:      userID,
				FirstName:   "John",
				LastName:    "Doe",
				DisplayName: "John Doe",
			},
		},
	}

	resp, err := h.Handler(context.Background(), Command{UserID: userID})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if resp.HasMfa {
		t.Error("expected HasMfa to be false")
	}
	if resp.Email != "user@example.com" {
		t.Errorf("expected email 'user@example.com', got '%s'", resp.Email)
	}
	if resp.FirstName != "John" {
		t.Errorf("expected firstName 'John', got '%s'", resp.FirstName)
	}
	if resp.DisplayName != "John Doe" {
		t.Errorf("expected displayName 'John Doe', got '%s'", resp.DisplayName)
	}
	if resp.MfaMethod != nil {
		t.Errorf("expected mfaMethod nil, got '%v'", resp.MfaMethod)
	}
}

func TestHandler_ReturnsProfileWithMfa(t *testing.T) {
	userID := uuid.New()
	appID := uuid.New()
	mfaMethod := "auth_app"

	h := &Handler{
		repository: &mockRepository{
			user: &entities.ApplicationUser{
				ID:                 userID,
				ApplicationID:      appID,
				Email:              "user@example.com",
				IsActive:           true,
				Preferred2FAMethod: &mfaMethod,
			},
			profile: &entities.UserProfile{
				UserID:      userID,
				FirstName:   "Jane",
				LastName:    "Smith",
				DisplayName: "Jane Smith",
			},
		},
	}

	resp, err := h.Handler(context.Background(), Command{UserID: userID})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if !resp.HasMfa {
		t.Error("expected HasMfa to be true")
	}
	if resp.MfaMethod == nil || *resp.MfaMethod != "auth_app" {
		t.Errorf("expected mfaMethod 'auth_app', got '%v'", resp.MfaMethod)
	}
}

func TestHandler_UserNotFound(t *testing.T) {
	h := &Handler{
		repository: &mockRepository{
			user: nil,
		},
	}

	_, err := h.Handler(context.Background(), Command{UserID: uuid.New()})
	if err == nil {
		t.Fatal("expected error for user not found")
	}
}
