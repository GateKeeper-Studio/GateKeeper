package listroles

import (
	"net/http"
	"strconv"

	"github.com/gate-keeper/internal/infra/database/repositories"
	http_router "github.com/gate-keeper/internal/presentation/http"
	"github.com/go-chi/chi"
	"github.com/google/uuid"
	"github.com/jackc/pgx/v5/pgxpool"
)

const defaultPage = 1
const defaultPageSize = 10

type Endpoint struct {
	DbPool *pgxpool.Pool
}

func (c *Endpoint) Http(writter http.ResponseWriter, request *http.Request) {
	tenantIDString := chi.URLParam(request, "tenantID")
	tenantIdUUID, err := uuid.Parse(tenantIDString)

	if err != nil {
		panic(err)
	}

	applicationIDString := chi.URLParam(request, "applicationID")
	applicationIdUUID, err := uuid.Parse(applicationIDString)

	if err != nil {
		panic(err)
	}

	page := defaultPage
	pageSize := defaultPageSize

	if p := request.URL.Query().Get("page"); p != "" {
		if parsed, err := strconv.Atoi(p); err == nil && parsed > 0 {
			page = parsed
		}
	}

	if ps := request.URL.Query().Get("pageSize"); ps != "" {
		if parsed, err := strconv.Atoi(ps); err == nil && parsed > 0 && parsed <= 100 {
			pageSize = parsed
		}
	}

	query := Query{
		TenantID:      tenantIdUUID,
		ApplicationID: applicationIdUUID,
		Page:          page,
		PageSize:      pageSize,
	}

	params := repositories.ParamsRs[Query, *Response, Handler]{
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
