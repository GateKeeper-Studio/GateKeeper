package accountgeneratebackupcodes

// Response contains the one-time-use backup codes.
// These codes are shown ONLY ONCE and must be stored securely by the user.
type Response struct {
	Codes   []string `json:"codes"`
	Message string   `json:"message"`
}
