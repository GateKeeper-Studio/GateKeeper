package googlelogin

import (
	"github.com/google/uuid"
)

type Command struct {
	OauthProviderId           uuid.UUID `json:"oauthProviderId"`
	ClientState               string    `json:"clientState"`
	ClientCodeChallengeMethod string    `json:"clientCodeChallengeMethod"`
	ClientCodeChallenge       string    `json:"clientCodeChallenge"`
	ClientScope               string    `json:"clientScope"`
	ClientResponseType        string    `json:"clientResponseType"`
	ClientRedirectUri         string    `json:"clientRedirectUri"`
	// OIDC nonce for ID Token binding
	ClientNonce string `json:"clientNonce"`
}
