package services

import "github.com/gate-keeper/internal/domain/entities"

// StrongAmrValues are AMR (Authentication Methods References) values from RFC 8176
// that indicate strong authentication was performed by the external provider.
var StrongAmrValues = map[string]bool{
	"mfa":    true, // Multi-factor authentication
	"otp":    true, // One-time password (TOTP/HOTP)
	"hwk":    true, // Hardware key (proof-of-possession of hardware-secured key)
	"swk":    true, // Software key
	"sms":    true, // SMS-based verification
	"pin":    true, // PIN-based verification
	"fpt":    true, // Fingerprint biometric
	"face":   true, // Facial recognition biometric
	"iris":   true, // Iris biometric
	"retina": true, // Retina biometric
	"vbm":    true, // Voice biometric
	"pop":    true, // Proof-of-possession
	"sc":     true, // Smart card
}

// MfaPolicyContext carries all the information needed by the policy engine
// to decide whether local MFA should be enforced.
type MfaPolicyContext struct {
	// AuthProvider is the authentication method used (e.g., "email", "google", "github").
	AuthProvider string

	// AmrClaims is the list of AMR values extracted from the provider's ID token.
	// Empty for providers that do not support OIDC amr (e.g. GitHub).
	AmrClaims []string

	// User is the authenticated user. If the user has no MFA configured,
	// MFA cannot be enforced regardless of policy.
	User *entities.TenantUser

	// Application holds the application-level settings that control MFA behavior.
	Application *entities.Application
}

// MfaPolicyDecision is the outcome produced by the policy engine.
type MfaPolicyDecision struct {
	// RequiresMfa indicates whether local MFA should be enforced.
	RequiresMfa bool

	// Reason is a human-readable explanation of why MFA is or is not required.
	Reason string
}

// EvaluateMfaRequirement applies the adaptive MFA policy rules in order:
//
//  1. If the application has RequiresHighSecurity enabled, MFA is ALWAYS required
//     (regardless of external provider MFA status).
//  2. If the user has no preferred 2FA method configured, MFA cannot be enforced.
//  3. If the user authenticated via a local credential (email/password), MFA is
//     required when configured (this is already handled by the login handler).
//  4. For external providers: if the AMR claims contain a strong authentication
//     indicator, local MFA is NOT required (the provider already enforced MFA).
//  5. Otherwise, local MFA IS required for external provider logins.
func EvaluateMfaRequirement(ctx MfaPolicyContext) MfaPolicyDecision {
	// Rule 1: Application-level high-security override
	if ctx.Application != nil && ctx.Application.RequiresHighSecurity {
		// Even if the provider did MFA, the application demands local MFA too.
		if ctx.User.Preferred2FAMethod == nil {
			return MfaPolicyDecision{
				RequiresMfa: false,
				Reason:      "Application requires high security but user has no MFA method configured",
			}
		}
		return MfaPolicyDecision{
			RequiresMfa: true,
			Reason:      "Application requires high security — local MFA is always enforced",
		}
	}

	// Rule 2: User has no MFA configured → skip
	if ctx.User == nil || ctx.User.Preferred2FAMethod == nil {
		return MfaPolicyDecision{
			RequiresMfa: false,
			Reason:      "User has no MFA method configured",
		}
	}

	// Rule 3: Local (email/password) login — defer to the existing login handler logic.
	// This function is meant to be called from OAuth callback handlers, so this
	// branch acts as a safeguard.
	if ctx.AuthProvider == "email" || ctx.AuthProvider == "" {
		return MfaPolicyDecision{
			RequiresMfa: true,
			Reason:      "Local credential login — MFA enforced by login handler",
		}
	}

	// Rule 4: External provider — check AMR claims for strong auth
	if hasStrongAmr(ctx.AmrClaims) {
		return MfaPolicyDecision{
			RequiresMfa: false,
			Reason:      "External provider already performed strong authentication (AMR check passed)",
		}
	}

	// Rule 5: External provider without strong AMR — require local MFA
	return MfaPolicyDecision{
		RequiresMfa: true,
		Reason:      "External provider did not perform strong authentication — local MFA required",
	}
}

// hasStrongAmr returns true if at least one of the AMR values is considered
// a strong authentication method.
func hasStrongAmr(amrClaims []string) bool {
	for _, amr := range amrClaims {
		if StrongAmrValues[amr] {
			return true
		}
	}
	return false
}
