package listusersessions

import "github.com/google/uuid"

type SessionResponse struct {
	ID        uuid.UUID `json:"id"`
	UserID    uuid.UUID `json:"userId"`
	ExpiresAt string    `json:"expiresAt"`
	CreatedAt string    `json:"createdAt"`
	IsActive  bool      `json:"isActive"`
}

type Response struct {
	Data []SessionResponse `json:"data"`
}
