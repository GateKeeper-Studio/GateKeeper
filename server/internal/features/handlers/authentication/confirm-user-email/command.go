package confirmuseremail

import "github.com/google/uuid"

type Command struct {
	Token               string    `json:"token" validate:"required"`
	Email               string    `json:"email" validate:"required,email"`
	ApplicationID       uuid.UUID `json:"applicationId" validate:"required"`
	CodeChallengeMethod string    `json:"codeChallengeMethod" validate:"required"`
	ResponseType        string    `json:"responseType" validate:"required"`
	Scope               string    `json:"scope" validate:"required"`
	State               string    `json:"state" validate:"required"`
	CodeChallenge       string    `json:"codeChallenge" validate:"required"`
	RedirectUri         string    `json:"redirectUri" validate:"required"`
	Nonce               string    `json:"nonce"`
}
