package reauthenticate

type Response struct {
	StepUpToken string `json:"stepUpToken"`
	ExpiresIn   int    `json:"expiresIn"` // seconds
}
