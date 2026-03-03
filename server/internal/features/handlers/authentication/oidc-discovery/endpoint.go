package oidcdiscovery

import (
	"net/http"
	"os"

	http_router "github.com/gate-keeper/internal/presentation/http"
)

type Endpoint struct{}

// OIDCDiscoveryResponse represents the OIDC Discovery document
// as defined in OpenID Connect Discovery 1.0
type OIDCDiscoveryResponse struct {
	Issuer                            string   `json:"issuer"`
	AuthorizationEndpoint             string   `json:"authorization_endpoint"`
	TokenEndpoint                     string   `json:"token_endpoint"`
	UserinfoEndpoint                  string   `json:"userinfo_endpoint"`
	JwksURI                           string   `json:"jwks_uri"`
	ResponseTypesSupported            []string `json:"response_types_supported"`
	SubjectTypesSupported             []string `json:"subject_types_supported"`
	IDTokenSigningAlgValuesSupported  []string `json:"id_token_signing_alg_values_supported"`
	ScopesSupported                   []string `json:"scopes_supported"`
	TokenEndpointAuthMethodsSupported []string `json:"token_endpoint_auth_methods_supported"`
	ClaimsSupported                   []string `json:"claims_supported"`
	CodeChallengeMethodsSupported     []string `json:"code_challenge_methods_supported"`
	GrantTypesSupported               []string `json:"grant_types_supported"`
}

func (e *Endpoint) Http(writer http.ResponseWriter, request *http.Request) {
	issuer := os.Getenv("ISSUER_URL")
	if issuer == "" {
		issuer = "https://proxymity.tech/guard"
	}

	baseURL := os.Getenv("BASE_URL")
	if baseURL == "" {
		baseURL = "http://localhost:8080"
	}

	response := OIDCDiscoveryResponse{
		Issuer:                            issuer,
		AuthorizationEndpoint:             baseURL + "/v1/auth/authorize",
		TokenEndpoint:                     baseURL + "/v1/auth/sign-in",
		UserinfoEndpoint:                  baseURL + "/v1/auth/userinfo",
		JwksURI:                           baseURL + "/.well-known/jwks.json",
		ResponseTypesSupported:            []string{"code"},
		SubjectTypesSupported:             []string{"public"},
		IDTokenSigningAlgValuesSupported:  []string{"HS256"},
		ScopesSupported:                   []string{"openid", "profile", "email", "offline_access"},
		TokenEndpointAuthMethodsSupported: []string{"client_secret_post"},
		ClaimsSupported: []string{
			"sub", "iss", "aud", "exp", "iat", "nbf", "jti",
			"name", "given_name", "family_name", "email",
			"nonce", "auth_time",
		},
		CodeChallengeMethodsSupported: []string{"S256", "plain"},
		GrantTypesSupported:           []string{"authorization_code", "refresh_token"},
	}

	http_router.SendJson(writer, response, http.StatusOK)
}
