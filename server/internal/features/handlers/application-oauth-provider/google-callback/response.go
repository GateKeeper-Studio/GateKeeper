package googlecallback

import "github.com/google/uuid"

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
