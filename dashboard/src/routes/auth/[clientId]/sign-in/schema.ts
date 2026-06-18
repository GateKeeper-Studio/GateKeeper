import { z } from 'zod';

export const formSchema = z.object({
	email: z.email('Invalid e-mail address'),
	password: z.string().min(2, 'Password must be at least 2 characters')
});
