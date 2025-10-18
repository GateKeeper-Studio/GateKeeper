package getproviderdatabyid

import (
	"time"

	"github.com/google/uuid"
)

type Response struct {
	ID            uuid.UUID  `json:"id"`
	Name          string     `json:"name"`
	Enabled       bool       `json:"enabled"`
	ApplicationID uuid.UUID  `json:"applicationId"`
	CreatedAt     time.Time  `json:"createdAt"`
	UpdatedAt     *time.Time `json:"updatedAt"`
	ClientID      string     `json:"clientId"`
	ClientSecret  string     `json:"clientSecret"`
	RedirectURI   string     `json:"redirectUri"`
}
