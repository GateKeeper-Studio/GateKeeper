import { z } from 'zod';

export const formSchema = z.object({
	displayName: z.string().min(2, 'Display name must be at least 2 characters'),
	firstName: z
		.string()
		.min(2, 'First name must be at least 2 characters')
		.max(50, 'First name must be at most 50 characters'),
	lastName: z
		.string()
		.min(2, 'Last name must be at least 2 characters')
		.max(50, 'Last name must be at most 50 characters'),
	email: z.email('Invalid e-mail address'),
	password: z.string().min(8, 'Password must be at least 8 characters')
});
