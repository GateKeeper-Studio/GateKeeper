package application_utils

import (
	"encoding/base64"
	"os"
	"strings"

	"github.com/gate-keeper/internal/domain/entities"
	"github.com/go-webauthn/webauthn/webauthn"
)

// WebAuthnUser implements the webauthn.User interface.
type WebAuthnUser struct {
	User        *entities.TenantUser
	Profile     *entities.UserProfile
	Credentials []entities.MfaWebauthnCredentials
}

func (w *WebAuthnUser) WebAuthnID() []byte {
	return w.User.ID[:]
}

func (w *WebAuthnUser) WebAuthnName() string {
	return w.User.Email
}

func (w *WebAuthnUser) WebAuthnDisplayName() string {
	if w.Profile != nil && w.Profile.DisplayName != "" {
		return w.Profile.DisplayName
	}
	return w.User.Email
}

func (w *WebAuthnUser) WebAuthnIcon() string {
	return ""
}

func (w *WebAuthnUser) WebAuthnCredentials() []webauthn.Credential {
	creds := make([]webauthn.Credential, 0, len(w.Credentials))
	for _, c := range w.Credentials {
		credIDBytes, err := base64.StdEncoding.DecodeString(c.CredentialID)
		if err != nil {
			continue
		}
		pubKeyBytes, err := base64.StdEncoding.DecodeString(c.PublicKey)
		if err != nil {
			continue
		}
		creds = append(creds, webauthn.Credential{
			ID:        credIDBytes,
			PublicKey: pubKeyBytes,
			Flags: webauthn.CredentialFlags{
				BackupEligible: c.BackupEligible,
				BackupState:    c.BackupState,
			},
			Authenticator: webauthn.Authenticator{
				SignCount: c.SignCount,
			},
		})
	}
	return creds
}

// NewWebAuthn creates a WebAuthn instance from environment variables.
// Required env vars: WEBAUTHN_RPID, WEBAUTHN_RPORIGIN, WEBAUTHN_RP_DISPLAY_NAME
// WEBAUTHN_RPORIGIN accepts a comma-separated list of allowed origins.
func NewWebAuthn() (*webauthn.WebAuthn, error) {
	origins := strings.Split(os.Getenv("WEBAUTHN_RPORIGIN"), ",")
	for i := range origins {
		origins[i] = strings.TrimSpace(origins[i])
	}

	return webauthn.New(&webauthn.Config{
		RPDisplayName: os.Getenv("WEBAUTHN_RP_DISPLAY_NAME"),
		RPID:          os.Getenv("WEBAUTHN_RPID"),
		RPOrigins:     origins,
	})
}
