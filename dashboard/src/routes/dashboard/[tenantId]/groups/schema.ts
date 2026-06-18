import { z } from 'zod';

export const groupFormSchema = z.object({
	name: z
		.string()
		.trim()
		.min(1, 'Name is required.')
		.max(128, 'Name must have at most 128 characters.'),
	description: z
		.string()
		.trim()
		.max(500, 'Description must have at most 500 characters.')
		.optional()
		.or(z.literal(''))
});

export type GroupFormValues = z.infer<typeof groupFormSchema>;
