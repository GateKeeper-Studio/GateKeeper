import { defineMDSveXConfig as defineConfig } from 'mdsvex';
import { fileURLToPath } from 'node:url';
import { createHighlighter } from 'shiki';

import { remarkCallouts } from './docs-tooling/remark-callouts.js';
import { rehypeDocsHeadings } from './docs-tooling/rehype-headings.js';

const THEMES = { light: 'github-light', dark: 'github-dark' };

const LANGS = [
	'bash',
	'json',
	'javascript',
	'typescript',
	'svelte',
	'html',
	'rust',
	'sql',
	'toml',
	'yaml',
	'http',
	'diff',
	'text'
];

/** @type {Promise<import('shiki').Highlighter> | undefined} */
let highlighterPromise;
const getHighlighter = () =>
	(highlighterPromise ??= createHighlighter({ themes: Object.values(THEMES), langs: LANGS }));

/**
 * mdsvex splices the highlighter's return value into the component as **raw Svelte markup**,
 * so `{` and `}` in a code sample (JSON, Rust generics, JS objects) would be parsed as Svelte
 * expressions. Shiki already escapes `<`, `>` and `&`, and its own markup contains no braces,
 * so entity-escaping every brace is lossless — the browser decodes them back on parse.
 *
 * mdsvex does this for its own Prism highlighter (`escape_svelty`) but deliberately skips code
 * blocks when a custom highlighter is supplied: escaping becomes our job.
 *
 * @param {string} html
 */
const escapeBraces = (html) => html.replace(/\{/g, '&#123;').replace(/\}/g, '&#125;');

/**
 * Mermaid source travels to the browser base64-encoded. Diagram syntax is full of quotes,
 * newlines, braces and `-->` arrows — base64's alphabet contains none of them, so the string
 * survives the Svelte parser and HTML attribute decoding untouched.
 *
 * @param {string} source
 */
const encode = (source) => Buffer.from(source, 'utf8').toString('base64');

export default defineConfig({
	// `.md` is deliberately NOT registered: it would turn every stray README in the project
	// into a Svelte component. Docs pages are `+page.svx`.
	extensions: ['.svx'],

	// Wraps every .svx file. The named exports in its module script are what make mdsvex add
	// `import * as Components from '<layout>'` to each compiled page.
	layout: {
		_: fileURLToPath(new URL('./src/lib/components/docs/mdsvex-layout.svelte', import.meta.url))
	},

	// Smartypants rewrites `--` to an en-dash and straightens quotes. That silently corrupts
	// every CLI flag we document (`--bin create_admin`) and any inline JSON.
	smartypants: false,

	// NOTE: mdsvex bundles the old unified/unist v2 stack, so the modern micromark-based
	// plugins (remark-gfm, rehype-slug, …) do not work here. These two are plain attachers
	// written against the old tree shape. GFM tables and strikethrough already work natively.
	remarkPlugins: [remarkCallouts],
	rehypePlugins: [rehypeDocsHeadings],

	highlight: {
		highlighter: async (code, lang, meta) => {
			// ```mermaid is not code to be highlighted — it's a diagram. `Components.` is
			// required: mdsvex only rewrites bare tag names on hast element nodes, and this
			// string is injected as raw markup.
			if (lang === 'mermaid') {
				return `<Components.Mermaid encoded="${encode(code)}" />`;
			}

			const shiki = await getHighlighter();
			const known = !!lang && shiki.getLoadedLanguages().includes(lang);
			const language = known ? lang : 'text';

			const html = shiki.codeToHtml(code, {
				lang: language,
				themes: THEMES,
				// Emit --shiki-light / --shiki-dark custom properties instead of baking in one
				// theme; layout.css picks the right one under `.dark`, with no JS and no flash.
				defaultColor: false,
				transformers: [
					{
						pre(node) {
							// Shiki sets tabindex="0", which trips Svelte's a11y warning on <pre>.
							delete node.properties.tabindex;
						}
					}
				]
			});

			// ```ts title="src/auth.ts" renders a filename header above the block.
			const title = /title="([^"]+)"/.exec(meta ?? '')?.[1];
			const attrs = `data-lang="${language}"${title ? ` data-title="${title}"` : ''}`;

			// `not-prose` keeps @tailwindcss/typography's pre/code styles off Shiki's output;
			// the wrapper is also the positioning context for the copy button.
			return `<div class="docs-code not-prose" ${attrs}>${escapeBraces(html)}</div>`;
		}
	}
});
