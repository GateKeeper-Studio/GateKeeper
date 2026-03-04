package githubcallback

import (
	application_utils "github.com/gate-keeper/internal/features/utils"
	"github.com/google/uuid"
)

type ServiceResponse struct {
	RedirectURL               string
	UserData                  *GitHubUserData
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

type GitHubUserData struct {
	Login     string `json:"login"`
	ID        int    `json:"id"`
	AvatarURL string `json:"avatar_url"`
	Name      string `json:"name"`
	Email     string `json:"email"`
}
