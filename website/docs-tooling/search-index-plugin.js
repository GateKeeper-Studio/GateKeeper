import { readdir, readFile } from 'node:fs/promises';
import path from 'node:path';

import { slugify } from './slug.js';

const VIRTUAL_ID = 'virtual:docs-search-index';
const RESOLVED_ID = '\0' + VIRTUAL_ID;

const ROUTES_DIR = path.resolve('src/routes');
const DOCS_DIR = path.resolve('src/routes/docs');

/**
 * Docs frontmatter is only ever `key: value`, so a full YAML parser would be overkill.
 *
 * @param {string} raw
 */
function splitFrontmatter(raw) {
	const match = /^---\r?\n([\s\S]*?)\r?\n---\r?\n?/.exec(raw);
	if (!match) return { data: /** @type {Record<string, string>} */ ({}), body: raw };

	/** @type {Record<string, string>} */
	const data = {};
	for (const line of match[1].split('\n')) {
		const pair = /^([A-Za-z0-9_-]+)\s*:\s*(.*)$/.exec(line.trim());
		if (pair) data[pair[1]] = pair[2].replace(/^['"]|['"]$/g, '');
	}

	return { data, body: raw.slice(match[0].length) };
}

/**
 * Markdown → plain searchable text. Code fences are dropped: matching a user's query against
 * a Shiki-highlighted `<span>` soup produces noise, not results.
 *
 * @param {string} markdown
 */
function toPlainText(markdown) {
	return markdown
		.replace(/```[\s\S]*?```/g, ' ')
		.replace(/<[^>]+>/g, ' ')
		.replace(/!\[[^\]]*\]\([^)]*\)/g, ' ')
		.replace(/\[([^\]]+)\]\([^)]*\)/g, '$1')
		.replace(/^\s*>\s?\[![a-z]+\]\s*/gim, '')
		.replace(/[#*_`~|>-]/g, ' ')
		.replace(/\s+/g, ' ')
		.trim();
}

/**
 * One record per page, plus one per h2/h3 so search can deep-link to a section. Anchors are
 * built with the same `slugify` the rehype plugin uses, so they always resolve.
 */
async function buildIndex() {
	/** @type {import('fs').Dirent[]} */
	let entries;
	try {
		entries = await readdir(DOCS_DIR, { recursive: true, withFileTypes: true });
	} catch {
		return [];
	}

	const index = [];

	for (const entry of entries) {
		if (!entry.isFile() || entry.name !== '+page.svx') continue;

		// `parentPath` replaced the older `path` in Node 20.12; fall back for older runtimes.
		const dir =
			entry.parentPath ?? /** @type {{ path: string }} */ (/** @type {unknown} */ (entry)).path;
		const raw = await readFile(path.join(dir, entry.name), 'utf8');
		const { data, body } = splitFrontmatter(raw);

		const url = '/' + path.relative(ROUTES_DIR, dir).split(path.sep).join('/');
		const title = data.title ?? url;
		const section = data.section ?? '';

		// The page itself: summary text is everything before the first h2.
		index.push({
			id: url,
			url,
			title,
			section,
			heading: null,
			text: toPlainText(body.split(/^##\s/m)[0]).slice(0, 400)
		});

		const headings = [...body.matchAll(/^(#{2,3})\s+(.+)$/gm)];
		headings.forEach((match, i) => {
			const heading = match[2].trim();
			const anchor = slugify(heading);
			const start = (match.index ?? 0) + match[0].length;
			const end = i + 1 < headings.length ? (headings[i + 1].index ?? body.length) : body.length;

			index.push({
				id: `${url}#${anchor}`,
				url: `${url}#${anchor}`,
				title,
				section,
				heading,
				text: toPlainText(body.slice(start, end)).slice(0, 600)
			});
		});
	}

	return index;
}

/**
 * Exposes the docs search index as `virtual:docs-search-index`.
 *
 * Building it here rather than shipping the raw markdown to the browser keeps the client bundle
 * free of the full docs corpus — only the stripped text needed to match a query is sent.
 *
 * @returns {import('vite').Plugin}
 */
export function docsSearchIndex() {
	return {
		name: 'docs-search-index',

		resolveId(id) {
			if (id === VIRTUAL_ID) return RESOLVED_ID;
		},

		async load(id) {
			if (id !== RESOLVED_ID) return;
			return `export default ${JSON.stringify(await buildIndex())};`;
		},

		configureServer(server) {
			/** @param {string} file */
			const reload = (file) => {
				if (!file.startsWith(DOCS_DIR) || !file.endsWith('.svx')) return;

				const graph = server.environments?.client?.moduleGraph ?? server.moduleGraph;
				const module = graph?.getModuleById?.(RESOLVED_ID);
				if (module) graph.invalidateModule(module);

				(server.hot ?? server.ws).send({ type: 'full-reload' });
			};

			server.watcher.on('change', reload);
			server.watcher.on('add', reload);
			server.watcher.on('unlink', reload);
		}
	};
}
