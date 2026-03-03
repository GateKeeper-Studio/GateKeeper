package authorize

type Response struct {
	AuthorizationCode   string `json:"authorizationCode"`
	RedirectUri         string `json:"redirectUri"`
	State               string `json:"state"`
	CodeChallenge       string `json:"codeChallenge"`
	CodeChallengeMethod string `json:"codeChallengeMethod"`
	ResponseType        string `json:"responseType"`
	Scope               string `json:"scope"`
}
