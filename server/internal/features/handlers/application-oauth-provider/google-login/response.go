package googlelogin

import "github.com/google/uuid"

type ServiceResponse struct {
	State               string
	ClientID            string
	Scope               string
	RedirectURI         string
	CodeChallenge       string
	CodeChallengeMethod string
	ApplicationID       uuid.UUID
}

type Response struct {
	Url   string `json:"url"`
	State string `json:"state"`
}
