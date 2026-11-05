-- label/description/options move from scalar text to JSONB keyed by locale.
-- Existing single-language data is preserved under the 'en' key. Postgres
-- forbids a subquery inside an ALTER COLUMN TYPE ... USING transform, so this
-- goes through temporary columns instead of a single ALTER ... TYPE ... USING.
ALTER TABLE tenant_custom_fields
  ADD COLUMN label_jsonb JSONB,
  ADD COLUMN description_jsonb JSONB,
  ADD COLUMN options_jsonb JSONB;

UPDATE tenant_custom_fields
SET
  label_jsonb = jsonb_build_object('en', label),
  description_jsonb = CASE WHEN description IS NULL THEN NULL ELSE jsonb_build_object('en', description) END,
  options_jsonb = (
    SELECT jsonb_agg(jsonb_build_object('en', opt))
    FROM unnest(options) AS opt
  );

ALTER TABLE tenant_custom_fields
  DROP COLUMN label,
  DROP COLUMN description,
  DROP COLUMN options;

ALTER TABLE tenant_custom_fields
  RENAME COLUMN label_jsonb TO label;
ALTER TABLE tenant_custom_fields
  RENAME COLUMN description_jsonb TO description;
ALTER TABLE tenant_custom_fields
  RENAME COLUMN options_jsonb TO options;

ALTER TABLE tenant_custom_fields ALTER COLUMN label SET NOT NULL;
