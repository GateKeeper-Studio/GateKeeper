package createtenantuser

import (
	"net/http"

	"github.com/gate-keeper/internal/infra/database/repositories"
	http_router "github.com/gate-keeper/internal/presentation/http"
	"github.com/go-chi/chi"
	"github.com/google/uuid"
	"github.com/jackc/pgx/v5/pgxpool"
)

type Endpoint struct {
	DbPool *pgxpool.Pool
}

func (c *Endpoint) Http(writter http.ResponseWriter, request *http.Request) {
	tenantIDString := chi.URLParam(request, "tenantID")
	tenantIdUUID, err := uuid.Parse(tenantIDString)

	if err != nil {
		panic(err)
	}

	var requestBody RequestBody

	if err := http_router.ParseBodyToSchema(&requestBody, request); err != nil {
		panic(err)
	}

	command := Command{
		TenantID:              tenantIdUUID,
		ApplicationID:         requestBody.ApplicationID,
		DisplayName:           requestBody.DisplayName,
		FirstName:             requestBody.FirstName,
		LastName:              requestBody.LastName,
		Email:                 requestBody.Email,
		IsEmailConfirmed:      requestBody.IsEmailConfirmed,
		TemporaryPasswordHash: requestBody.TemporaryPasswordHash,
		Roles:                 requestBody.Roles,
	}

	params := repositories.ParamsRs[Command, *Response, Handler]{
		DbPool:  c.DbPool,
		New:     New,
		Request: command,
	}

	response, err := repositories.WithTransactionRs(request.Context(), params)

	if err != nil {
		panic(err)
	}

	http_router.SendJson(writter, response, http.StatusCreated)
}
