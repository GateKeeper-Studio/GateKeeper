-- Lossy: keeps only the 'en' translation of each field, which is the best
-- any down-migration can do once per-language data has been entered.
ALTER TABLE tenant_custom_fields
  ADD COLUMN label_text VARCHAR(150),
  ADD COLUMN description_text VARCHAR(500),
  ADD COLUMN options_text TEXT[];

UPDATE tenant_custom_fields
SET
  label_text = label ->> 'en',
  description_text = description ->> 'en',
  options_text = CASE
    WHEN options IS NULL THEN NULL
    ELSE ARRAY(SELECT jsonb_array_elements(options) ->> 'en')
  END;

ALTER TABLE tenant_custom_fields
  DROP COLUMN label,
  DROP COLUMN description,
  DROP COLUMN options;

ALTER TABLE tenant_custom_fields RENAME COLUMN label_text TO label;
ALTER TABLE tenant_custom_fields RENAME COLUMN description_text TO description;
ALTER TABLE tenant_custom_fields RENAME COLUMN options_text TO options;

ALTER TABLE tenant_custom_fields ALTER COLUMN label SET NOT NULL;
