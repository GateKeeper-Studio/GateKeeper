// This app is a static SPA served by the identity provider (UI_DIR). SvelteKit
// server-only files would build but never run there — or worse, a `+page.server.ts`
// load would silently fail in production. Refuse them early.
import { readdirSync } from 'node:fs';
import { join } from 'node:path';

const FORBIDDEN = /^(\+page\.server|\+layout\.server|\+server|hooks\.server)\.(ts|js)$/;

function walk(dir) {
	return readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
		const path = join(dir, entry.name);
		return entry.isDirectory() ? walk(path) : [path];
	});
}

const offenders = walk('src').filter((path) => FORBIDDEN.test(path.split(/[/\\]/).pop()));
if (offenders.length > 0) {
	console.error('Server-only SvelteKit files are not allowed in this static app:');
	for (const path of offenders) console.error(`  ${path}`);
	console.error('Move the logic to a universal load (+page.ts) or to the Rust server.');
	process.exit(1);
}
