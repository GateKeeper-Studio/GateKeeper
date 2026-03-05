package createtenant

type Command struct {
	Name        string  `json:"name" validate:"required"`
	Description *string `json:"description" validate:"omitempty"`
}
