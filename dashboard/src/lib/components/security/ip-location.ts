import type { IpLocationView } from '$lib/services/admin/security';

// How a point on the security maps is drawn, worst first. Derived only from what the
// server reports, so the map and the tables agree:
//   - blocked: the IP is under an active block right now;
//   - attack:  it reached HIGH/CRITICAL risk, or the risk engine refused it;
//   - warning: MEDIUM risk, or any wrong password;
//   - normal:  everything else.
export const CATEGORIES = ['blocked', 'attack', 'warning', 'normal'] as const;
export type Category = (typeof CATEGORIES)[number];

export function categoryOf(point: IpLocationView): Category {
	if (point.blocked) return 'blocked';
	if (
		point.max_risk_level === 'CRITICAL' ||
		point.max_risk_level === 'HIGH' ||
		point.blocked_attempts > 0
	) {
		return 'attack';
	}
	if (point.max_risk_level === 'MEDIUM' || point.failed_attempts > 0) return 'warning';
	return 'normal';
}

// Tailwind classes per category: the marker dot, and the small badge in its popup.
export const DOT_CLASS: Record<Category, string> = {
	blocked: 'bg-red-700 ring-2 ring-red-300 dark:ring-red-900',
	attack: 'bg-red-500',
	warning: 'bg-amber-500',
	normal: 'bg-emerald-500'
};

export const BADGE_CLASS: Record<Category, string> = {
	blocked: 'bg-red-100 text-red-800 dark:bg-red-950 dark:text-red-300',
	attack: 'bg-orange-100 text-orange-800 dark:bg-orange-950 dark:text-orange-300',
	warning: 'bg-amber-100 text-amber-800 dark:bg-amber-950 dark:text-amber-300',
	normal: 'bg-emerald-100 text-emerald-800 dark:bg-emerald-950 dark:text-emerald-300'
};

// Dot diameter in pixels: grows with the attempt count, logarithmically, so one noisy
// IP cannot cover a continent.
export function dotSize(attempts: number): number {
	return 10 + Math.min(14, Math.round(Math.log2(Math.max(1, attempts)) * 3));
}

export function placeOf(point: IpLocationView): string | null {
	const parts = [point.city, point.region, point.country_code].filter(Boolean);
	return parts.length ? parts.join(', ') : null;
}

export function networkOf(point: IpLocationView): string | null {
	if (point.isp && point.asn) return `${point.isp} · AS${point.asn}`;
	if (point.isp) return point.isp;
	return point.asn ? `AS${point.asn}` : null;
}

// The windows the map offers, in days.
export const RANGES = { '1d': 1, '7d': 7, '30d': 30, '90d': 90 } as const;
export type Range = keyof typeof RANGES;
