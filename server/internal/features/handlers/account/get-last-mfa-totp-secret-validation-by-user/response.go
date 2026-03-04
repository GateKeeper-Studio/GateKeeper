package accountgetlastmfatotpsecret

import "time"

type Response struct {
	OtpUrl    string    `json:"otpUrl"`
	ExpiresAt time.Time `json:"expiresAt"`
}
