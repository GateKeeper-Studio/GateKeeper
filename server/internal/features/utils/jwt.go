package application_utils

import (
	"os"
	"time"

	"github.com/golang-jwt/jwt/v5"
	"github.com/google/uuid"
)

type JWTClaims struct {
	UserID      uuid.UUID
	FirstName   string
	LastName    string
	DisplayName string
	Email       string
	TenantID    uuid.UUID
}

// CreateToken creates an OAuth2 access token (JWT) with OIDC-compatible claims
func CreateToken(claims JWTClaims) (string, error) {
	return createTokenWithOptions(claims, nil, nil)
}

// CreateIDToken creates an OIDC ID Token with optional nonce claim
func CreateIDToken(claims JWTClaims, nonce *string, audience string) (string, error) {
	return createIDTokenWithOptions(claims, nonce, audience)
}

func createTokenWithOptions(claims JWTClaims, nonce *string, audience interface{}) (string, error) {
	key := []byte(os.Getenv("JWT_SECRET"))
	issuer := os.Getenv("ISSUER_URL")
	if issuer == "" {
		issuer = "https://proxymity.tech/guard"
	}

	now := time.Now()

	mappedClaims := jwt.MapClaims{
		// OIDC standard claims
		"sub":         claims.UserID.String(),
		"oid":         claims.UserID.String(), // legacy alias
		"given_name":  claims.FirstName,
		"family_name": claims.LastName,
		"name":        claims.DisplayName,
		"email":       claims.Email,
		"org_id":      claims.TenantID.String(),
		// JWT registered claims
		"aud": "https://proxymity.tech/guard",
		"exp": now.Add(time.Minute * 15).Unix(),
		"iat": now.Unix(),
		"nbf": now.Unix(),
		"iss": issuer,
		"jti": uuid.New().String(),
	}

	if nonce != nil && *nonce != "" {
		mappedClaims["nonce"] = *nonce
	}

	token := jwt.NewWithClaims(jwt.SigningMethodHS256, mappedClaims)
	return token.SignedString(key)
}

func createIDTokenWithOptions(claims JWTClaims, nonce *string, audience string) (string, error) {
	key := []byte(os.Getenv("JWT_SECRET"))
	issuer := os.Getenv("ISSUER_URL")
	if issuer == "" {
		issuer = "https://proxymity.tech/guard"
	}

	now := time.Now()

	mappedClaims := jwt.MapClaims{
		// OIDC required claims for ID Token (RFC 7519 + OIDC Core 1.0)
		"iss":       issuer,
		"sub":       claims.UserID.String(),
		"aud":       audience,
		"exp":       now.Add(time.Minute * 15).Unix(),
		"iat":       now.Unix(),
		"auth_time": now.Unix(),
		"jti":       uuid.New().String(),
		// OIDC standard profile claims
		"given_name":  claims.FirstName,
		"family_name": claims.LastName,
		"name":        claims.DisplayName,
		"email":       claims.Email,
		"org_id":      claims.TenantID.String(),
	}

	if nonce != nil && *nonce != "" {
		mappedClaims["nonce"] = *nonce
	}

	token := jwt.NewWithClaims(jwt.SigningMethodHS256, mappedClaims)
	return token.SignedString(key)
}

func ValidateToken(jwtToken string) (bool, string, error) {
	key := []byte(os.Getenv("JWT_SECRET"))

	token, err := jwt.Parse(jwtToken, func(token *jwt.Token) (interface{}, error) {
		return key, nil
	})

	if err != nil {
		return false, "", err
	}

	claims, ok := token.Claims.(jwt.MapClaims)

	if !ok {
		return false, "", err
	}

	return token.Valid, claims["sub"].(string), nil
}

// ValidateTokenWithLeeway validates a JWT allowing recently-expired tokens (up to `leeway` past expiry).
// This is used by the token refresh endpoint so clients can obtain a new token even if the current one
// just expired.
func ValidateTokenWithLeeway(jwtToken string, leeway time.Duration) (bool, string, error) {
	key := []byte(os.Getenv("JWT_SECRET"))

	parser := jwt.NewParser(jwt.WithLeeway(leeway))

	token, err := parser.Parse(jwtToken, func(token *jwt.Token) (interface{}, error) {
		return key, nil
	})

	if err != nil {
		return false, "", err
	}

	claims, ok := token.Claims.(jwt.MapClaims)

	if !ok {
		return false, "", err
	}

	return token.Valid, claims["sub"].(string), nil
}

func DecodeToken(jwtToken string) (*JWTClaims, error) {
	key := []byte(os.Getenv("JWT_SECRET"))

	token, err := jwt.Parse(jwtToken, func(token *jwt.Token) (interface{}, error) {
		return key, nil
	})

	if err != nil {
		return nil, err
	}

	claims, ok := token.Claims.(jwt.MapClaims)

	if !ok {
		return nil, err
	}

	return &JWTClaims{
		UserID:      uuid.MustParse(claims["sub"].(string)),
		FirstName:   claims["given_name"].(string),
		LastName:    claims["family_name"].(string),
		DisplayName: claims["name"].(string),
		Email:       claims["email"].(string),
		TenantID:    uuid.MustParse(claims["org_id"].(string)),
	}, nil
}
