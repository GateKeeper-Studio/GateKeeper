package entities

import (
	"time"

	"github.com/google/uuid"
)

type ExternalOAuthState struct {
	ID                         uuid.UUID
	ProviderState              string
	ApplicationOAuthProviderID uuid.UUID

	ClientState               string
	ClientCodeChallengeMethod string
	ClientCodeChallenge       string
	ClientScope               string
	ClientResponseType        string
	ClientRedirectUri         string

	CreatedAt time.Time
}

func AddExternalOAuthState(
	state string,
	applicationOAuthProviderID uuid.UUID,
	clientState string,
	clientCodeChallengeMethod string,
	clientCodeChallenge string,
	clientScope string,
	clientResponseType string,
	clientRedirectUri string,
) *ExternalOAuthState {
	newId, err := uuid.NewV7()

	if err != nil {
		panic(err)
	}

	return &ExternalOAuthState{
		ID:                         newId,
		ProviderState:              state,
		ApplicationOAuthProviderID: applicationOAuthProviderID,
		ClientState:                clientState,
		ClientCodeChallengeMethod:  clientCodeChallengeMethod,
		ClientCodeChallenge:        clientCodeChallenge,
		ClientScope:                clientScope,
		ClientResponseType:         clientResponseType,
		ClientRedirectUri:          clientRedirectUri,
		CreatedAt:                  time.Now(),
	}
}
