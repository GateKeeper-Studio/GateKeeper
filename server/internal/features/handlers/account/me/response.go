package me

type MfaMethodItem struct {
	Type    string `json:"type"`
	Enabled bool   `json:"enabled"`
}

type Response struct {
	UserID      string          `json:"userId"`
	Email       string          `json:"email"`
	FirstName   string          `json:"firstName"`
	LastName    string          `json:"lastName"`
	DisplayName string          `json:"displayName"`
	PhoneNumber *string         `json:"phoneNumber"`
	Address     *string         `json:"address"`
	HasMfa      bool            `json:"hasMfa"`
	MfaMethod   *string         `json:"mfaMethod,omitempty"`
	MfaMethods  []MfaMethodItem `json:"mfaMethods"`
}
