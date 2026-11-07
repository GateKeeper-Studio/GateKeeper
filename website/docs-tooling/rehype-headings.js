import { slugify } from './slug.js';

/**
 * mdsvex bundles the old unified/unist v2 stack, so `rehype-slug` (unified 11) can't be
 * used here. This is the same job in a dozen lines, written as a plain attacher that
 * works on any unified version.
 *
 * Two outputs:
 *  1. stable `id` on every h2/h3, so anchors resolve in the prerendered HTML;
 *  2. the heading list pushed into `file.data.fm` (frontmatter). mdsvex serializes that
 *     into `export const metadata` and spreads it onto the layout, so the layout receives
 *     `headings` as a prop — a server-rendered TOC with no client-side DOM scan.
 *
 * The TOC has to be build-time: SvelteKit prerenders with `handleMissingId: 'fail'`, and
 * it checks `#hash` links against ids present in the *static* HTML. Ids generated after
 * hydration would be too late.
 *
 * @param {any} node
 * @param {(el: any) => void} fn
 */
function walk(node, fn) {
	if (node.type === 'element') fn(node);
	for (const child of node.children ?? []) walk(child, fn);
}

/**
 * @param {any} node
 * @returns {string}
 */
function textOf(node) {
	if (node.type === 'text') return node.value;
	return (node.children ?? []).map(textOf).join('');
}

export function rehypeDocsHeadings() {
	return (/** @type {any} */ tree, /** @type {any} */ file) => {
		/** @type {Array<{ depth: number, id: string, text: string }>} */
		const headings = [];
		/** @type {Map<string, number>} */
		const seen = new Map();

		walk(tree, (node) => {
			if (!/^h[23]$/.test(node.tagName)) return;

			const text = textOf(node).trim();
			if (!text) return;

			let id = slugify(text);
			// Two headings can share a title ("Fields", "Fields"); disambiguate the second.
			const count = seen.get(id) ?? 0;
			seen.set(id, count + 1);
			if (count > 0) id = `${id}-${count}`;

			node.properties = { ...(node.properties ?? {}), id };
			headings.push({ depth: Number(node.tagName[1]), id, text });
		});

		file.data.fm = { ...(file.data.fm ?? {}), headings };
	};
}
