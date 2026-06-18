import { z } from 'zod';

export const formSchema = z.object({
	code: z.string().min(6, 'Code must contain 6 characters').max(6, 'Code must contain 6 characters')
});
