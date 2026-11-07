/**
 * Heading → anchor id.
 *
 * Shared by the rehype plugin (which stamps the ids into the HTML) and the search-index
 * plugin (which builds `#anchor` links pointing at them). They MUST agree: SvelteKit
 * prerenders with `handleMissingId: 'fail'`, so a link to an id that was never emitted
 * fails the build.
 *
 * @param {string} text
 * @returns {string}
 */
export function slugify(text) {
	return text
		.toLowerCase()
		.trim()
		.replace(/[^\p{L}\p{N}\s-]/gu, '')
		.replace(/\s+/g, '-')
		.replace(/-{2,}/g, '-')
		.replace(/^-|-$/g, '');
}
