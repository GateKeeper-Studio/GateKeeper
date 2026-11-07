/**
 * The signed-in platform admin, as the server's `/session` endpoint returns it.
 * Carries no credential: the session itself is an httpOnly cookie that JavaScript
 * cannot read.
 */
export type AdminIdentity = {
	id: string;
	email: string;
	organizationId: string;
	/** Friendly label: the account name, else derived from the email. */
	name: string;
	/** 1–2 letter avatar fallback derived from {@link name}. */
	initials: string;
};
