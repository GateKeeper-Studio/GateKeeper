import { browser } from '$app/environment';

/** How long before the deadline we start warning the user. */
const WARN_BEFORE_MS = 2 * 60 * 1000;

/**
 * Stop offering "keep me signed in" when the absolute ceiling is this close: the
 * server would clamp the extension to a sliver and the user would be right back where
 * they started. Better to tell them plainly that the session is ending.
 */
const MIN_USEFUL_EXTENSION_MS = 30 * 1000;

/**
 * A hint from the relying party (`request_expires_at`) is attacker-influenceable — it
 * rides in on the query string. We clamp it so a tampered value can't make the page
 * promise a sign-in window that lasts all day. It only ever drives *display*; the real
 * enforcement lives at the RP's callback, which we cannot and should not second-guess.
 */
const MAX_PRE_AUTH_MS = 60 * 60 * 1000;

export type ExpiryPhase = 'dormant' | 'ok' | 'warning' | 'expired';

const sessionKey = (clientId: string) => `auth_session_${clientId}`;
const expiryKey = (clientId: string) => `auth_session_${clientId}_expires_at`;
const ceilingKey = (clientId: string) => `auth_session_${clientId}_absolute_expires_at`;

/** The session code the login flow is currently carrying, or '' if there is none. */
export function readSessionCode(clientId: string): string {
	if (!browser) return '';
	return sessionStorage.getItem(sessionKey(clientId)) || '';
}

/**
 * Persist the session code together with its deadlines.
 *
 * These travel as a unit: storing the code without its clock is what would let a page
 * navigation (sign-in → MFA) silently restart the countdown from zero and show the user
 * a comfortable "10 minutes left" for a session that has two.
 */
export function storeSession(
	clientId: string,
	sessionCode: string,
	expiresAt: string,
	absoluteExpiresAt: string
) {
	if (!browser) return;
	sessionStorage.setItem(sessionKey(clientId), sessionCode);
	sessionStorage.setItem(expiryKey(clientId), expiresAt);
	sessionStorage.setItem(ceilingKey(clientId), absoluteExpiresAt);
}

/** Drop the session and its clock — on success, on expiry, on giving up. */
export function clearSession(clientId: string) {
	if (!browser) return;
	sessionStorage.removeItem(sessionKey(clientId));
	sessionStorage.removeItem(expiryKey(clientId));
	sessionStorage.removeItem(ceilingKey(clientId));
}

function readStoredTime(key: string): number | null {
	if (!browser) return null;
	const raw = sessionStorage.getItem(key);
	if (!raw) return null;
	const parsed = Date.parse(raw);
	return Number.isNaN(parsed) ? null : parsed;
}

/**
 * A countdown to a sign-in deadline.
 *
 * Two deadlines actually run during a sign-in, and this models either:
 *
 *   * BEFORE the password is submitted, the clock belongs to the *relying party* — its
 *     OAuth state / PKCE verifier cookies. We can't extend that (we have never seen the
 *     code_verifier); recovery means asking the RP to start over.
 *   * AFTER the password is submitted, the clock is the identity provider's own
 *     `AuthSession`, and that one we can genuinely slide forward.
 *
 * Whichever it is, time is always computed as `deadline - Date.now()`, never by
 * decrementing a counter. Background tabs get their timers throttled to roughly once a
 * minute and a suspended laptop stops them dead; a decremented counter would quietly
 * over-report how much time is left, which is the one failure this whole feature exists
 * to prevent. Re-reading the wall clock also lets `visibilitychange` / `pageshow` snap
 * the display straight back to the truth when the user returns.
 */
export class SessionClock {
	/** Epoch ms of the idle deadline, or null when there is nothing to count. */
	expiresAt = $state<number | null>(null);

	/** Epoch ms of the absolute ceiling. Null when unknown (pre-auth has no ceiling). */
	absoluteExpiresAt = $state<number | null>(null);

	/** Wall clock, re-read on every tick and on every visibility change. */
	now = $state(Date.now());

	readonly remainingMs = $derived(
		this.expiresAt === null ? Number.POSITIVE_INFINITY : this.expiresAt - this.now
	);

	readonly phase = $derived<ExpiryPhase>(
		this.expiresAt === null
			? 'dormant'
			: this.remainingMs <= 0
				? 'expired'
				: this.remainingMs <= WARN_BEFORE_MS
					? 'warning'
					: 'ok'
	);

	/**
	 * Whether an extension would actually buy the user meaningful time. False once the
	 * ceiling is within reach — the server would refuse or clamp to almost nothing, and
	 * promising a refresh we know will fail is worse than saying so up front.
	 */
	readonly canExtend = $derived(
		this.absoluteExpiresAt === null
			? true
			: this.now < this.absoluteExpiresAt - MIN_USEFUL_EXTENSION_MS
	);

	/** `m:ss` for display. */
	readonly formatted = $derived.by(() => {
		const total = Math.max(0, Math.ceil(this.remainingMs / 1000));
		const minutes = Math.floor(total / 60);
		const seconds = total % 60;
		return `${minutes}:${String(seconds).padStart(2, '0')}`;
	});

	/**
	 * A coarse, human phrase for screen readers. The visible `m:ss` changes every second;
	 * announcing that would machine-gun assistive tech, so the live region says this
	 * instead and only changes a handful of times.
	 */
	readonly announcement = $derived.by(() => {
		const seconds = Math.max(0, Math.ceil(this.remainingMs / 1000));
		if (seconds <= 0) return 'Your sign-in session has expired.';
		if (seconds <= 10) return 'Your sign-in session expires in a few seconds.';
		if (seconds <= 30) return 'Your sign-in session expires in about 30 seconds.';
		if (seconds <= 60) return 'Your sign-in session expires in about a minute.';
		const minutes = Math.round(seconds / 60);
		return `Your sign-in session expires in about ${minutes} minutes.`;
	});

	/** Start counting to an explicit deadline (post-login: we know both bounds). */
	set(expiresAt: string | number, absoluteExpiresAt?: string | number | null) {
		this.expiresAt = typeof expiresAt === 'number' ? expiresAt : Date.parse(expiresAt);
		this.absoluteExpiresAt =
			absoluteExpiresAt == null
				? null
				: typeof absoluteExpiresAt === 'number'
					? absoluteExpiresAt
					: Date.parse(absoluteExpiresAt);
		this.now = Date.now();
	}

	/** Rehydrate from sessionStorage when landing on an MFA page mid-flow. */
	hydrateFromStorage(clientId: string) {
		const expiresAt = readStoredTime(expiryKey(clientId));
		if (expiresAt === null) return;
		this.expiresAt = expiresAt;
		this.absoluteExpiresAt = readStoredTime(ceilingKey(clientId));
		this.now = Date.now();
	}

	/**
	 * Adopt the relying party's `request_expires_at` hint (epoch seconds) for the
	 * pre-password clock. Ignores anything absent, unparseable, already past, or
	 * implausibly far out — a bad hint should leave the guard dormant (today's
	 * behavior), never grant a longer-looking window than the RP could really honor.
	 */
	adoptRequestExpiryHint(rawEpochSeconds: string | null) {
		if (!rawEpochSeconds) return;
		const seconds = Number(rawEpochSeconds);
		if (!Number.isFinite(seconds)) return;

		const deadline = seconds * 1000;
		const now = Date.now();
		if (deadline <= now) return;

		this.expiresAt = Math.min(deadline, now + MAX_PRE_AUTH_MS);
		// The RP's state has no renewable window — it either lives or it doesn't — so
		// there is no ceiling to report and nothing to slide.
		this.absoluteExpiresAt = null;
		this.now = now;
	}

	/** Force the clock dead, e.g. when the server answers 401 before the timer agrees. */
	expireNow() {
		this.expiresAt = Date.now();
		this.now = Date.now();
	}

	/**
	 * Drive the clock. Call from a `$effect` so the interval and listeners are torn down
	 * with the component. The 1s tick only refreshes `now`; correctness comes from
	 * re-reading the wall clock, not from the interval firing on time.
	 */
	start(): () => void {
		if (!browser) return () => {};

		const sync = () => (this.now = Date.now());
		const interval = setInterval(sync, 1000);

		// A throttled or suspended tab leaves `now` stale. These snap it back the instant
		// the user is looking again — `pageshow` covers bfcache restores, where the page
		// resumes with its old JS state intact.
		document.addEventListener('visibilitychange', sync);
		window.addEventListener('pageshow', sync);

		return () => {
			clearInterval(interval);
			document.removeEventListener('visibilitychange', sync);
			window.removeEventListener('pageshow', sync);
		};
	}
}
