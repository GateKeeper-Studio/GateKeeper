package accountupdateprofile

import "github.com/google/uuid"

type Command struct {
	UserID      uuid.UUID `json:"-"`
	FirstName   string    `json:"firstName" validate:"required"`
	LastName    string    `json:"lastName" validate:"required"`
	DisplayName string    `json:"displayName" validate:"required"`
	PhoneNumber *string   `json:"phoneNumber"`
	Address     *string   `json:"address"`
}
