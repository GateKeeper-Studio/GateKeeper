package userinfo

import (
	"net/http"

	application_utils "github.com/gate-keeper/internal/features/utils"
	http_router "github.com/gate-keeper/internal/presentation/http"
	"github.com/jackc/pgx/v5/pgxpool"
)

type Endpoint struct {
	DbPool *pgxpool.Pool
}

// UserInfoResponse is the OIDC UserInfo endpoint response.
// Spec: https://openid.net/specs/openid-connect-core-1_0.html#UserInfoResponse
type UserInfoResponse struct {
	Sub        string  `json:"sub"`
	Name       string  `json:"name"`
	GivenName  string  `json:"given_name"`
	FamilyName string  `json:"family_name"`
	Email      string  `json:"email"`
	Picture    *string `json:"picture,omitempty"`
}

// Http handles GET /v1/auth/userinfo
// Requires: Authorization: Bearer <access_token>
func (e *Endpoint) Http(writer http.ResponseWriter, request *http.Request) {
	authHeader := request.Header.Get("Authorization")
	if len(authHeader) < 8 || authHeader[:7] != "Bearer " {
		http.Error(writer, `{"error":"invalid_token","error_description":"Missing or malformed Bearer token"}`, http.StatusUnauthorized)
		return
	}

	tokenString := authHeader[7:]

	isValid, _, err := application_utils.ValidateToken(tokenString)
	if err != nil || !isValid {
		http.Error(writer, `{"error":"invalid_token","error_description":"Token validation failed"}`, http.StatusUnauthorized)
		return
	}

	claims, err := application_utils.DecodeToken(tokenString)
	if err != nil {
		http.Error(writer, `{"error":"invalid_token","error_description":"Could not decode token"}`, http.StatusUnauthorized)
		return
	}

	response := UserInfoResponse{
		Sub:        claims.UserID.String(),
		Name:       claims.DisplayName,
		GivenName:  claims.FirstName,
		FamilyName: claims.LastName,
		Email:      claims.Email,
	}

	http_router.SendJson(writer, response, http.StatusOK)
}
