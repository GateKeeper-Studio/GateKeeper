import { adminApi as api } from '../base/admin-api';
import { ApiError, type Result, type VoidResult } from '../types';

export type CustomFieldType = 'text' | 'number' | 'boolean' | 'date' | 'select';

// Mirrors the IdP's CustomFieldView (camelCase — that projection renames), returned
// by the create/get/list/update custom-field-definition endpoints. `label`/
// `description`/`options` are per-language now: `label`/`description` map
// locale -> text, and `options` is an ORDERED array of per-language choice
// objects (array order = display order end users see in the `<select>`;
// every entry's keys are the same set of locales the admin has translated
// that choice into — index alignment across languages is structural, since
// there's only one array).
export type CustomFieldDefinitionView = {
	id: string;
	tenantId: string;
	fieldKey: string;
	label: Record<string, string>;
	description: Record<string, string> | null;
	fieldType: CustomFieldType;
	options: Record<string, string>[] | null;
	isRequired: boolean;
	displayOrder: number;
	isActive: boolean;
	createdAt: string;
	updatedAt: string;
};

export async function listCustomFields(
	tenantId: string,
	fetchFn: typeof fetch = fetch
): Promise<Result<CustomFieldDefinitionView[]>> {
	try {
		const data = await api.get<{ fields: CustomFieldDefinitionView[] }>(
			`/tenants/${tenantId}/custom-fields`,
			fetchFn
		);
		return [data.fields, null];
	} catch (error) {
		return [null, error as ApiError];
	}
}

export async function getCustomField(
	tenantId: string,
	fieldId: string,
	fetchFn: typeof fetch = fetch
): Promise<Result<CustomFieldDefinitionView>> {
	try {
		const data = await api.get<CustomFieldDefinitionView>(
			`/tenants/${tenantId}/custom-fields/${fieldId}`,
			fetchFn
		);
		return [data, null];
	} catch (error) {
		return [null, error as ApiError];
	}
}

// What the create endpoint accepts (camelCase here, mapped to the IdP's snake_case
// Command below — that Command has no camelCase rename). `fieldKey`/`fieldType` are
// only ever sent at creation: both are immutable afterwards.
export type CreateCustomFieldInput = {
	fieldKey: string;
	label: Record<string, string>;
	description?: Record<string, string> | null;
	fieldType: CustomFieldType;
	options?: Record<string, string>[] | null;
	isRequired: boolean;
	displayOrder: number;
};

export async function createCustomField(
	tenantId: string,
	input: CreateCustomFieldInput
): Promise<Result<CustomFieldDefinitionView>> {
	try {
		const data = await api.post<CustomFieldDefinitionView>(`/tenants/${tenantId}/custom-fields`, {
			field_key: input.fieldKey,
			label: input.label,
			description: input.description ?? null,
			field_type: input.fieldType,
			options: input.options ?? null,
			is_required: input.isRequired,
			display_order: input.displayOrder
		});
		return [data, null];
	} catch (error) {
		return [null, error as ApiError];
	}
}

// The editable details of an existing field. `fieldKey`/`fieldType` are
// intentionally absent — the IdP refuses to change either after creation.
export type UpdateCustomFieldInput = {
	label: Record<string, string>;
	description?: Record<string, string> | null;
	options?: Record<string, string>[] | null;
	isRequired: boolean;
	displayOrder: number;
	isActive: boolean;
};

export async function updateCustomField(
	tenantId: string,
	fieldId: string,
	input: UpdateCustomFieldInput
): Promise<Result<CustomFieldDefinitionView>> {
	try {
		const data = await api.put<CustomFieldDefinitionView>(
			`/tenants/${tenantId}/custom-fields/${fieldId}`,
			{
				label: input.label,
				description: input.description ?? null,
				options: input.options ?? null,
				is_required: input.isRequired,
				display_order: input.displayOrder,
				is_active: input.isActive
			}
		);
		return [data, null];
	} catch (error) {
		return [null, error as ApiError];
	}
}

export async function deleteCustomField(tenantId: string, fieldId: string): Promise<VoidResult> {
	try {
		await api.delete(`/tenants/${tenantId}/custom-fields/${fieldId}`);
		return [null];
	} catch (error) {
		return [error as ApiError];
	}
}

// PUT …/custom-fields/reorder — sends the full ordered list of field IDs; the
// server reassigns display_order = index for each, atomically. Snake_case
// wire body (this endpoint's Command has no camelCase rename, matching its
// create/update siblings).
export async function reorderCustomFields(
	tenantId: string,
	fieldIds: string[]
): Promise<VoidResult> {
	try {
		await api.put(`/tenants/${tenantId}/custom-fields/reorder`, { field_ids: fieldIds });
		return [null];
	} catch (error) {
		return [error as ApiError];
	}
}

// PUT …/users/:userId/custom-field-values — full replace, deliberately kept
// separate from `updateTenantUser` so a name-only edit never wipes previously
// saved custom values (see update_tenant_user_custom_field_values on the API).
export async function updateUserCustomFieldValues(
	tenantId: string,
	userId: string,
	values: Record<string, unknown> | null
): Promise<Result<{ userId: string; customFieldValues: Record<string, unknown> | null }>> {
	try {
		const data = await api.put<{
			userId: string;
			customFieldValues: Record<string, unknown> | null;
		}>(`/tenants/${tenantId}/users/${userId}/custom-field-values`, { custom_field_values: values });
		return [data, null];
	} catch (error) {
		return [null, error as ApiError];
	}
}
