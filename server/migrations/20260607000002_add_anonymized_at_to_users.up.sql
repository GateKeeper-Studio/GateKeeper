-- Right to erasure / anonymization (WI-06). We anonymize rather than hard-delete so
-- referential history (e.g. audit attribution) survives without retaining PII.
-- `anonymized_at` marks the row as a tombstone; `legal_hold` blocks erasure when a
-- legal/contractual obligation requires the data to be retained.
ALTER TABLE "users" ADD COLUMN IF NOT EXISTS anonymized_at TIMESTAMPTZ NULL;
ALTER TABLE "users" ADD COLUMN IF NOT EXISTS legal_hold BOOLEAN NOT NULL DEFAULT false;
