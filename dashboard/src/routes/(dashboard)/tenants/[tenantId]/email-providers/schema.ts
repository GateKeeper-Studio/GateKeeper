import { z } from 'zod';

// Password is optional here so the same schema serves create and edit; the page
// enforces "required on create" separately (on edit, an empty password keeps the
// stored one).
export const formSchema = z.object({
	name: z.string().min(1, 'Name is required'),
	host: z.string().min(1, 'Host is required'),
	port: z
		.number({ message: 'Port must be a number' })
		.int('Port must be a whole number')
		.min(1, 'Port must be 1–65535')
		.max(65535, 'Port must be 1–65535'),
	username: z.string().min(1, 'Username is required'),
	password: z.string().optional(),
	fromEmail: z.email('Invalid from-email address'),
	fromName: z.string().optional(),
	encryption: z.enum(['starttls', 'tls', 'none']),
	isDefault: z.boolean(),
	isActive: z.boolean()
});

export type FormValues = z.infer<typeof formSchema>;
