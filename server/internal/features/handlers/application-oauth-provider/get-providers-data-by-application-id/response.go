package getprovidersdatabyapplicationid

import (
	"time"

	"github.com/google/uuid"
)

type ApplicationProvider struct {
	ID           uuid.UUID  `json:"id"`
	Name         string     `json:"name"`
	IsEnabled    bool       `json:"isEnabled"`
	ClientID     string     `json:"clientId"`
	ClientSecret string     `json:"clientSecret"`
	RedirectURI  string     `json:"redirectUri"`
	UpdatedAt    *time.Time `json:"updatedAt"`
	CreatedAt    time.Time  `json:"createdAt"`
}

type Response []ApplicationProvider
