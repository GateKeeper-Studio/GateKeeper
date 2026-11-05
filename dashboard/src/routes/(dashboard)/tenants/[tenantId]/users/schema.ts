import { z } from 'zod';

// The fields shared by both create and edit. Display name is optional (the IdP
// accepts null), the rest are required. Email shape is checked loosely — the IdP
// performs the authoritative validation.
const baseFields = {
	displayName: z.string().trim().max(75, 'Display name must have at most 75 characters.'),
	firstName: z.string().trim().min(1, 'First name is required.'),
	lastName: z.string().trim().min(1, 'Last name is required.')
};

// Create also collects the email and a temporary password the user must rotate on
// first sign-in, plus whether the email is already verified.
export const createUserFormSchema = z.object({
	...baseFields,
	email: z.string().trim().min(1, 'Email is required.').email('Enter a valid email address.'),
	isEmailVerified: z.boolean(),
	temporaryPassword: z
		.string()
		.min(8, 'Temporary password must have at least 8 characters.')
});

// Edit only touches the name fields — the IdP manages email and password through
// their own flows.
export const editUserFormSchema = z.object(baseFields);

export type CreateUserFormValues = z.infer<typeof createUserFormSchema>;
export type EditUserFormValues = z.infer<typeof editUserFormSchema>;
