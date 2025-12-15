package githublogin

import "github.com/google/uuid"

type ServiceResponse struct {
	State         string
	ClientID      string
	Scope         string
	RedirectURI   string
	ApplicationID uuid.UUID
}

type Response struct {
	Url   string `json:"url"`
	State string `json:"state"`
}
