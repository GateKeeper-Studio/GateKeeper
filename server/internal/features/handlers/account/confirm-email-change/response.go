package accountconfirmemailchange

type Response struct {
	Message  string `json:"message"`
	NewEmail string `json:"newEmail"`
}
