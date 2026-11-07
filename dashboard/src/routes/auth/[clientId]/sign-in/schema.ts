import { z } from 'zod';

export const formSchema = z.object({
	email: z.email('Invalid e-mail address'),
	password: z.string().min(2, 'Password must be at least 2 characters')
});

// Passwordless login submits the email alone.
export const passkeySchema = z.object({
	email: z.email('Invalid e-mail address')
});
