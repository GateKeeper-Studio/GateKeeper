package accountlistmfamethods

type MfaMethodItem struct {
	Type    string `json:"type"`
	Enabled bool   `json:"enabled"`
}

type Response struct {
	PreferredMethod *string         `json:"preferredMethod"`
	Methods         []MfaMethodItem `json:"methods"`
}
