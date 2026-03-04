package accountupdateprofile

type Response struct {
	FirstName   string  `json:"firstName"`
	LastName    string  `json:"lastName"`
	DisplayName string  `json:"displayName"`
	PhoneNumber *string `json:"phoneNumber"`
	Address     *string `json:"address"`
}
