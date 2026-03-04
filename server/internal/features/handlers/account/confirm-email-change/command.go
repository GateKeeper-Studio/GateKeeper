package accountconfirmemailchange

type Command struct {
	Token     string `json:"token" validate:"required"`
	IPAddress string `json:"-"`
	UserAgent string `json:"-"`
}
