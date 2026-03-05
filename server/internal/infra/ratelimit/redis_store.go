package ratelimit

import (
	"context"
	"fmt"
	"time"

	"github.com/redis/go-redis/v9"
)

// slidingWindowScript is an atomic Lua script that implements a sliding-window
// counter using a Redis sorted set. Each member is a unique timestamp-based
// entry; entries outside the current window are pruned on every call.
//
// KEYS[1] – the rate-limit key
// ARGV[1] – max allowed requests (limit)
// ARGV[2] – window size in milliseconds
// ARGV[3] – current time in milliseconds
// Returns 1 if the request is allowed, 0 if it exceeds the limit.
const slidingWindowScript = `
local key    = KEYS[1]
local limit  = tonumber(ARGV[1])
local window = tonumber(ARGV[2])
local now    = tonumber(ARGV[3])
local oldest = now - window

redis.call('ZREMRANGEBYSCORE', key, '-inf', oldest)

local count = tonumber(redis.call('ZCARD', key))
if count < limit then
    local member = tostring(now) .. '-' .. tostring(redis.call('INCR', key .. ':seq'))
    redis.call('ZADD', key, now, member)
    redis.call('PEXPIRE', key, window)
    return 1
end
return 0
`

// RedisStore implements Store using Redis sorted sets for a sliding-window
// rate limiter. All operations are executed atomically via a Lua script.
type RedisStore struct {
	client *redis.Client
	script *redis.Script
}

// NewRedisStore returns a RedisStore backed by the provided Redis client.
func NewRedisStore(client *redis.Client) *RedisStore {
	return &RedisStore{
		client: client,
		script: redis.NewScript(slidingWindowScript),
	}
}

// Allow returns true when the request key is within the rate limit.
func (s *RedisStore) Allow(ctx context.Context, key string, limit int, window time.Duration) (bool, error) {
	now := time.Now().UnixMilli()
	windowMs := window.Milliseconds()

	result, err := s.script.Run(ctx, s.client, []string{key}, limit, windowMs, now).Int()
	if err != nil {
		return false, fmt.Errorf("ratelimit: redis script: %w", err)
	}

	return result == 1, nil
}
