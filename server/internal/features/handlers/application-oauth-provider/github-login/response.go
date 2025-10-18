package githublogin

type ServiceResponse struct {
	State       string
	ClientID    string
	Scope       string
	RedirectURI string
}

type Response struct {
	Url   string `json:"url"`
	State string `json:"state"`
}
