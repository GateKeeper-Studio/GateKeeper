package listapplications

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

	query := Query{TenantID: tenantIdUUID}

	params := repositories.ParamsRs[Query, *[]Response, Handler]{
		DbPool:  c.DbPool,
		New:     New,
		Request: query,
	}

	response, err := repositories.WithTransactionRs(request.Context(), params)

	if err != nil {
		panic(err)
	}

	http_router.SendJson(writter, response, http.StatusOK)
}
