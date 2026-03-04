package application_utils

import (
	"encoding/base64"
	"encoding/json"
	"strings"
)

// AmrClaims represents the relevant claims extracted from an OIDC ID token.
type AmrClaims struct {
	// Amr is the Authentication Methods References array (RFC 8176).
	Amr []string `json:"amr"`
}

// ExtractAmrFromIDToken decodes a JWT ID token (without signature verification,
// since the token was already validated during the OAuth token exchange) and
// extracts the "amr" claim.
//
// Returns an empty slice if the token is empty, malformed, or does not contain
// an amr claim. This is safe because the token has already been verified by the
// provider's token endpoint via the authorization code exchange.
func ExtractAmrFromIDToken(idToken string) []string {
	if idToken == "" {
		return nil
	}

	parts := strings.Split(idToken, ".")
	if len(parts) != 3 {
		return nil
	}

	// Decode the payload (second part)
	payload, err := base64.RawURLEncoding.DecodeString(parts[1])
	if err != nil {
		return nil
	}

	var claims AmrClaims
	if err := json.Unmarshal(payload, &claims); err != nil {
		return nil
	}

	return claims.Amr
}
