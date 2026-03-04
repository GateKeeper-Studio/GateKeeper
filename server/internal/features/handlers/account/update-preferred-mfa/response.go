package accountupdatepreferredmfa

type Response struct {
	Message         string  `json:"message"`
	PreferredMethod *string `json:"preferredMethod"`
}
