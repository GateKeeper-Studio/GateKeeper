CREATE TABLE IF NOT EXISTS "subscriptions" (
    id UUID PRIMARY KEY,
    plan_id UUID NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NULL,
    -- RESTRICT: a plan that still has subscriptions cannot be deleted, so the
    -- delete-plan use case can surface a clean "plan in use" conflict.
    CONSTRAINT fk_subscription_plan FOREIGN KEY (plan_id)
        REFERENCES "subscription_plans" (id) ON DELETE RESTRICT
);

CREATE INDEX idx_subscriptions_plan_id ON subscriptions (plan_id);
