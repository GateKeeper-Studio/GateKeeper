package accountrefreshtoken

type Response struct {
	AccessToken string `json:"accessToken"`
	ExpiresIn   int64  `json:"expiresIn"` // seconds until expiration
}
