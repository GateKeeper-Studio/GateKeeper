package googlecallback

import (
	application_utils "github.com/gate-keeper/internal/features/utils"
	"github.com/google/uuid"
)

type ServiceResponse struct {
	RedirectURL               string
	UserData                  *GoogleUserData
	OauthProviderID           uuid.UUID
	ClientState               string
	AuthorizationCode         string
	ClientCodeChallengeMethod string
	ClientCodeChallenge       string
	ClientScope               string
	ClientResponseType        string
	ClientRedirectUri         string
	ClientNonce               *string
	MfaRequired               bool
	MfaChallenge              *application_utils.MfaChallengeResult
}

type GoogleUserData struct {
	ID            string `json:"sub"`
	Name          string `json:"name"`
	GivenName     string `json:"given_name"`
	FamilyName    string `json:"family_name"`
	Picture       string `json:"picture"`
	Email         string `json:"email"`
	EmailVerified bool   `json:"email_verified"`
	Locale        string `json:"locale"`
}
