import { z } from 'zod';
import type { CustomFieldDefinitionLike } from './types';

/**
 * Builds a runtime Zod schema from a tenant's custom field definitions, so
 * submitted values can go through the same `validate()` helper as every other
 * form in this dashboard (see `$lib/utils/form.ts`).
 */
export function buildCustomFieldsZodSchema(definitions: CustomFieldDefinitionLike[]) {
	const shape: Record<string, z.ZodTypeAny> = {};

	for (const def of definitions) {
		let fieldSchema: z.ZodTypeAny;

		switch (def.fieldType) {
			case 'text': {
				let text = z.string().max(1000, 'Must be 1000 characters or fewer');
				if (def.isRequired) text = text.min(1, 'This field is required');
				fieldSchema = text;
				break;
			}
			case 'number':
				fieldSchema = z.number({ message: 'Must be a number' });
				break;
			case 'boolean':
				fieldSchema = z.boolean();
				break;
			case 'date':
				fieldSchema = z
					.string()
					.regex(/^\d{4}-\d{2}-\d{2}$/, 'Must be a valid date')
					.refine((v) => !Number.isNaN(new Date(v).getTime()), 'Must be a valid date');
				break;
			case 'select': {
				const options = def.options ?? [];
				fieldSchema = options.length > 0 ? z.enum(options as [string, ...string[]]) : z.string();
				break;
			}
			default:
				fieldSchema = z.unknown();
		}

		shape[def.fieldKey] = def.isRequired ? fieldSchema : fieldSchema.optional();
	}

	return z.object(shape);
}
