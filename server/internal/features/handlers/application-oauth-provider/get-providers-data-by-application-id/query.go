package getprovidersdatabyapplicationid

import "github.com/google/uuid"

type Query struct {
	ApplicationID uuid.UUID
}
