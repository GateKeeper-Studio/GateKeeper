package githubcallback

type Command struct {
	Code  string
	State string
	// StoredState      string
	// StoredProviderID uuid.UUID
}

type githubRequestBody struct {
	ClientID     string `json:"client_id"`
	ClientSecret string `json:"client_secret"`
	Code         string `json:"code"`
}

type githubResponsePayload struct {
	AccessToken string `json:"access_token"`
	Scope       string `json:"scope"`
	TokenType   string `json:"token_type"`
}
