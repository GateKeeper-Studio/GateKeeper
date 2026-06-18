-- Remove fixed plans only when no subscription references them.
DELETE FROM subscription_plans AS p
WHERE p.id IN (
    '00000000-0000-0000-0000-000000000101',
    '00000000-0000-0000-0000-000000000102',
    '00000000-0000-0000-0000-000000000103'
)
AND NOT EXISTS (
    SELECT 1 FROM subscriptions AS s WHERE s.plan_id = p.id
);
