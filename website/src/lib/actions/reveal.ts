import type { Action } from 'svelte/action';

export type RevealOptions = {
	/** Delay in ms before the element animates in (used for stagger). */
	delay?: number;
	/** IntersectionObserver threshold. */
	threshold?: number;
	/** Only reveal once (default true). */
	once?: boolean;
};

/**
 * `use:reveal` — fades + slides an element into view when it scrolls into the
 * viewport. Pairs with the `.reveal` base styles in layout.css: the node starts
 * translated/transparent and gets `data-revealed` when it intersects.
 *
 * Honors `prefers-reduced-motion`: reduced-motion users get the revealed state
 * immediately with no transition.
 */
export const reveal: Action<HTMLElement, RevealOptions | undefined> = (node, options) => {
	const opts: Required<RevealOptions> = {
		delay: 0,
		threshold: 0.15,
		once: true,
		...options
	};

	node.classList.add('reveal');
	if (opts.delay) node.style.transitionDelay = `${opts.delay}ms`;

	const reduce =
		typeof window !== 'undefined' && window.matchMedia('(prefers-reduced-motion: reduce)').matches;

	if (reduce || typeof IntersectionObserver === 'undefined') {
		node.setAttribute('data-revealed', '');
		return { destroy() {} };
	}

	const observer = new IntersectionObserver(
		(entries) => {
			for (const entry of entries) {
				if (entry.isIntersecting) {
					node.setAttribute('data-revealed', '');
					if (opts.once) observer.unobserve(node);
				} else if (!opts.once) {
					node.removeAttribute('data-revealed');
				}
			}
		},
		{ threshold: opts.threshold }
	);

	observer.observe(node);

	return {
		destroy() {
			observer.disconnect();
		}
	};
};
