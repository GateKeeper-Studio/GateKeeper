package createtenant

type Command struct {
	Name               string  `json:"name" validate:"required"`
	Description        *string `json:"description" validate:"omitempty"`
	PasswordHashSecret string  `json:"passwordHashSecret" validate:"required,min=32,max=258"`
}
