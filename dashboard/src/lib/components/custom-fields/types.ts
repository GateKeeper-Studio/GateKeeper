import type { CustomFieldType } from '$lib/services/admin/custom-fields';

// Minimal shape the dynamic renderer needs. Both the admin management view
// (CustomFieldDefinitionView) and the public sign-up page's own client-auth-data
// projection satisfy this — neither needs to expose more than this to render.
export type CustomFieldDefinitionLike = {
	fieldKey: string;
	label: string;
	description?: string | null;
	fieldType: CustomFieldType;
	options?: string[] | null;
	isRequired: boolean;
	displayOrder?: number;
};
