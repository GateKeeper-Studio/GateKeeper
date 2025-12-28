package googlecallback

type Command struct {
	Code  string
	State string
	// StoredState      string
	// StoredProviderID uuid.UUID
}

type googleRequestBody struct {
	ClientID     string `json:"client_id"`
	ClientSecret string `json:"client_secret"`
	Code         string `json:"code"`
	GrantType    string `json:"grant_type,omitempty"`
	RedirectURI  string `json:"redirect_uri,omitempty"`
	CodeVerifier string `json:"code_verifier,omitempty"`
}

type googleResponsePayload struct {
	AccessToken  string `json:"access_token"`
	IdToken      string `json:"id_token"`	
	RefreshToken string `json:"refresh_token"`
	ExpiresIn    int    `json:"expires_in"`
}
