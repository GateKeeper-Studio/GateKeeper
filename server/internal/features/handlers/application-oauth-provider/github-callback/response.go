package githubcallback

import "github.com/google/uuid"

type ServiceResponse struct {
	RedirectURL               string
	UserData                  *GitHubUserData
	OauthProviderID           uuid.UUID
	ClientState               string
	ClientCodeChallengeMethod string
	ClientCodeChallenge       string
	ClientScope               string
	ClientResponseType        string
	ClientRedirectUri         string
}

type GitHubUserData struct {
	Login     string `json:"login"`
	ID        int    `json:"id"`
	AvatarURL string `json:"avatar_url"`
	Name      string `json:"name"`
	Email     string `json:"email"`
}
