/**
 * Error thrown by the API client for any non-2xx response.
 *
 * Mirrors the shape the GateKeeper Go server returns in its error bodies
 * (`{ message, title }`) so the UI can surface friendly messages.
 */
export class ApiError extends Error {
	readonly status: number;
	readonly data: {
		error?: string;
		message?: string;
		title?: string;
		errors?: { message?: string }[];
	};

	constructor(
		status: number,
		data: { error?: string; message?: string; title?: string; errors?: { message?: string }[] }
	) {
		// The server uses several body shapes: `{ message }`, `{ error }`, and
		// `{ errors: [{ message }] }` (e.g. BadRequest). Pull the most specific
		// human-readable message out of whichever one came back.
		super(
			data?.message ??
				data?.error ??
				data?.errors?.[0]?.message ??
				`Request failed with status ${status}`
		);
		this.name = 'ApiError';
		this.status = status;
		this.data = data ?? {};
	}
}

/** Tuple result for calls that return a payload: `[data, null]` or `[null, error]`. */
export type Result<T> = [T, null] | [null, ApiError];

/** Tuple result for calls that only signal success/failure: `[null]` or `[error]`. */
export type VoidResult = [ApiError | null];
