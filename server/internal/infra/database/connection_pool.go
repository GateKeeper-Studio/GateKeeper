package database

import (
	"context"
	"fmt"

	"github.com/jackc/pgx/v5/pgxpool"
)

func NewConnectionPool() (*pgxpool.Pool, error) {
	ctx := context.Background()

	pool, err := pgxpool.New(ctx, fmt.Sprintf("user=%s password=%s host=%s port=%s dbname=%s",
		"gatekeeper",
		"gatekeeper",
		"localhost",
		"5432",
		"gatekeeper",
	))

	if err != nil {
		fmt.Println("Unable to connect to database ", err)
		return nil, err
	}

	return pool, nil
}
