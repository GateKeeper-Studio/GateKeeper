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
	/**
	 * The organization requires two-step verification and this admin has not set it
	 * up. The server then opens only the enrollment endpoints; the console sends the
	 * admin to `/login/two-step`.
	 */
	mfaSetupRequired: boolean;
	/**
	 * The highest version of the console's guided tour this admin has finished or
	 * closed (0: never). The console shows the tour on first access below its own.
	 */
	consoleTourVersion: number;
};
