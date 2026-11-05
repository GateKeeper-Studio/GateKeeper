import { z } from 'zod';

// Mirrors the backend's own slug rule (create_tenant_custom_field's field_key
// check) so the form fails the same way the API would, before a round trip.
const FIELD_KEY_PATTERN = /^[a-z][a-z0-9_]{0,63}$/;

// Validates the admin's field-*definition* form (create/edit a custom field).
// Not to be confused with `buildCustomFieldsZodSchema` in
// `$lib/components/custom-fields/schema.ts`, which validates end-user
// *values* against an existing set of definitions.
//
// `label`/`description`/`options` are per-language maps now, so this schema
// is built per-form-instance around the tenant's `defaultLanguage` — the one
// locale every field must have translated (mirrors the server-side
// `validate_localized_text`/`validate_localized_options` requirement).
export function buildCustomFieldFormSchema(defaultLanguage: string) {
	const localizedText = (max: number, message: string) =>
		z.record(z.string(), z.string().trim().max(max, message));

	return z
		.object({
			fieldKey: z
				.string()
				.trim()
				.min(1, 'Field key is required.')
				.max(64, 'Field key must be 64 characters or fewer.')
				.regex(
					FIELD_KEY_PATTERN,
					'Use lowercase letters, numbers, and underscores, starting with a letter.'
				),
			label: localizedText(150, 'Label must be 150 characters or fewer.').superRefine(
				(map, ctx) => {
					if (!(map[defaultLanguage] ?? '').trim()) {
						ctx.addIssue({
							code: 'custom',
							message: 'Label is required in the tenant default language.'
						});
					}
				}
			),
			description: localizedText(500, 'Description must be 500 characters or fewer.'),
			fieldType: z.enum(['text', 'number', 'boolean', 'date', 'select']),
			options: z
				.array(localizedText(100, 'Options must be 100 characters or fewer.'))
				.max(50, 'At most 50 options are allowed.'),
			isRequired: z.boolean(),
			isActive: z.boolean()
		})
		.superRefine((data, ctx) => {
			if (data.fieldType !== 'select') return;
			const requiredValues = data.options.map((o) => (o[defaultLanguage] ?? '').trim());
			if (requiredValues.filter(Boolean).length === 0) {
				ctx.addIssue({ code: 'custom', path: ['options'], message: 'Add at least one option.' });
				return;
			}
			if (requiredValues.some((v) => !v)) {
				ctx.addIssue({
					code: 'custom',
					path: ['options'],
					message: 'Every option needs text in the tenant default language.'
				});
				return;
			}
			if (new Set(requiredValues).size !== requiredValues.length) {
				ctx.addIssue({ code: 'custom', path: ['options'], message: 'Options must be unique.' });
			}
		});
}

export type CustomFieldFormValues = z.infer<ReturnType<typeof buildCustomFieldFormSchema>>;
