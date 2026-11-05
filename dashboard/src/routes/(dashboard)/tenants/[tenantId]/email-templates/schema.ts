import { z } from 'zod';

// Mirrors the server-side validation on update/preview (subject 1–300, body
// 1–100000). The server also enforces the placeholder allow-list and rejects
// <script>/<mj-raw>; those are reported as API errors, not pre-validated here.
export const formSchema = z.object({
	subject: z.string().min(1, 'Subject is required').max(300, 'Subject is too long'),
	bodyMjml: z.string().min(1, 'Body is required').max(100000, 'Body is too long')
});

export type FormValues = z.infer<typeof formSchema>;
