package entities

import (
	"time"

	"github.com/google/uuid"
)

type ApplicationAuthorizationCode struct {
	ID                  uuid.UUID
	ApplicationID       uuid.UUID
	ExpiresAt           time.Time
	Code                string
	TenantUserId   uuid.UUID
	RedirectUri         string
	CodeChallenge       string
	CodeChallengeMethod string
	Nonce               *string
	Scope               *string
}

func CreateApplicationAuthorizationCode(applicationID, tenantUserID uuid.UUID, redirectUri, codeChallenge, codeChallegeMethod string, nonce *string, scope *string) (*ApplicationAuthorizationCode, error) {
	userId, err := uuid.NewV7()

	if err != nil {
		return nil, err
	}

	return &ApplicationAuthorizationCode{
		ID:                  userId,
		ApplicationID:       applicationID,
		ExpiresAt:           time.Now().UTC().Add(time.Minute * 5), // 5 minutes
		Code:                GenerateRandomString(128),
		TenantUserId:   tenantUserID,
		RedirectUri:         redirectUri,
		CodeChallenge:       codeChallenge,
		CodeChallengeMethod: codeChallegeMethod,
		Nonce:               nonce,
		Scope:               scope,
	}, nil
}
