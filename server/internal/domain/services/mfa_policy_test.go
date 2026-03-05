package services

import (
	"testing"

	"github.com/gate-keeper/internal/domain/entities"
	"github.com/google/uuid"
)

func ptrString(s string) *string {
	return &s
}

func makeUser(preferred2FA *string) *entities.TenantUser {
	return &entities.TenantUser{
		ID:                 uuid.New(),
		TenantID:           uuid.New(),
		Email:              "test@example.com",
		IsActive:           true,
		IsEmailConfirmed:   true,
		Preferred2FAMethod: preferred2FA,
	}
}

func makeApp(requiresHighSecurity bool) *entities.Application {
	return &entities.Application{
		ID:                   uuid.New(),
		Name:                 "Test App",
		RequiresHighSecurity: requiresHighSecurity,
		HasMfaAuthApp:        true,
		HasMfaEmail:          true,
	}
}

func TestEvaluateMfaRequirement_HighSecurity_AlwaysRequiresMfa(t *testing.T) {
	user := makeUser(ptrString("totp"))
	app := makeApp(true)

	decision := EvaluateMfaRequirement(MfaPolicyContext{
		AuthProvider: "google",
		AmrClaims:    []string{"mfa", "otp"}, // Even with strong AMR
		User:         user,
		Application:  app,
	})

	if !decision.RequiresMfa {
		t.Errorf("Expected MFA required with RequiresHighSecurity=true, got false: %s", decision.Reason)
	}
}

func TestEvaluateMfaRequirement_HighSecurity_NoMfaConfigured(t *testing.T) {
	user := makeUser(nil) // No MFA configured
	app := makeApp(true)

	decision := EvaluateMfaRequirement(MfaPolicyContext{
		AuthProvider: "github",
		User:         user,
		Application:  app,
	})

	if decision.RequiresMfa {
		t.Errorf("Expected MFA not required when user has no MFA configured, got true: %s", decision.Reason)
	}
}

func TestEvaluateMfaRequirement_NoMfaConfigured_SkipsMfa(t *testing.T) {
	user := makeUser(nil)
	app := makeApp(false)

	decision := EvaluateMfaRequirement(MfaPolicyContext{
		AuthProvider: "google",
		User:         user,
		Application:  app,
	})

	if decision.RequiresMfa {
		t.Errorf("Expected MFA not required for user without MFA, got true: %s", decision.Reason)
	}
}

func TestEvaluateMfaRequirement_NilUser(t *testing.T) {
	app := makeApp(false)

	decision := EvaluateMfaRequirement(MfaPolicyContext{
		AuthProvider: "google",
		User:         nil,
		Application:  app,
	})

	if decision.RequiresMfa {
		t.Errorf("Expected MFA not required for nil user, got true: %s", decision.Reason)
	}
}

func TestEvaluateMfaRequirement_LocalLogin_AlwaysRequiresMfa(t *testing.T) {
	user := makeUser(ptrString("totp"))
	app := makeApp(false)

	decision := EvaluateMfaRequirement(MfaPolicyContext{
		AuthProvider: "email",
		User:         user,
		Application:  app,
	})

	if !decision.RequiresMfa {
		t.Errorf("Expected MFA required for local login, got false: %s", decision.Reason)
	}
}

func TestEvaluateMfaRequirement_ExternalProvider_StrongAmr_SkipsMfa(t *testing.T) {
	user := makeUser(ptrString("totp"))
	app := makeApp(false)

	testCases := []struct {
		name string
		amr  []string
	}{
		{"mfa claim", []string{"pwd", "mfa"}},
		{"otp claim", []string{"pwd", "otp"}},
		{"hwk claim", []string{"hwk"}},
		{"fpt claim", []string{"fpt"}},
		{"pop claim", []string{"pop"}},
		{"sc claim", []string{"sc"}},
	}

	for _, tc := range testCases {
		t.Run(tc.name, func(t *testing.T) {
			decision := EvaluateMfaRequirement(MfaPolicyContext{
				AuthProvider: "google",
				AmrClaims:    tc.amr,
				User:         user,
				Application:  app,
			})

			if decision.RequiresMfa {
				t.Errorf("Expected MFA not required with strong AMR %v, got true: %s", tc.amr, decision.Reason)
			}
		})
	}
}

func TestEvaluateMfaRequirement_ExternalProvider_WeakAmr_RequiresMfa(t *testing.T) {
	user := makeUser(ptrString("email"))
	app := makeApp(false)

	testCases := []struct {
		name string
		amr  []string
	}{
		{"password only", []string{"pwd"}},
		{"empty amr", []string{}},
		{"nil amr", nil},
		{"unknown method", []string{"unknown"}},
	}

	for _, tc := range testCases {
		t.Run(tc.name, func(t *testing.T) {
			decision := EvaluateMfaRequirement(MfaPolicyContext{
				AuthProvider: "github",
				AmrClaims:    tc.amr,
				User:         user,
				Application:  app,
			})

			if !decision.RequiresMfa {
				t.Errorf("Expected MFA required with weak/no AMR %v, got false: %s", tc.amr, decision.Reason)
			}
		})
	}
}

func TestEvaluateMfaRequirement_EmptyAuthProvider_TreatedAsLocal(t *testing.T) {
	user := makeUser(ptrString("totp"))
	app := makeApp(false)

	decision := EvaluateMfaRequirement(MfaPolicyContext{
		AuthProvider: "",
		User:         user,
		Application:  app,
	})

	if !decision.RequiresMfa {
		t.Errorf("Expected MFA required with empty auth provider (treated as local), got false: %s", decision.Reason)
	}
}

func TestHasStrongAmr(t *testing.T) {
	tests := []struct {
		name     string
		amr      []string
		expected bool
	}{
		{"nil", nil, false},
		{"empty", []string{}, false},
		{"pwd only", []string{"pwd"}, false},
		{"mfa present", []string{"pwd", "mfa"}, true},
		{"otp present", []string{"otp"}, true},
		{"hwk present", []string{"hwk"}, true},
		{"fpt biometric", []string{"fpt"}, true},
		{"face biometric", []string{"face"}, true},
		{"multiple strong", []string{"mfa", "otp", "hwk"}, true},
		{"mixed", []string{"pwd", "unknown", "otp"}, true},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := hasStrongAmr(tt.amr)
			if result != tt.expected {
				t.Errorf("hasStrongAmr(%v) = %v, want %v", tt.amr, result, tt.expected)
			}
		})
	}
}
