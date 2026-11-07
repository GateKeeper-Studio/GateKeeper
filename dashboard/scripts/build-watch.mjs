// `npm run build:watch`: rebuild build/ whenever the sources change, for the Rust
// server to serve (UI_DIR). No dev server runs; Node only compiles.
//
// Each rebuild is a fresh `vite build` into a temporary directory, swapped in only
// when it succeeded. So the server never serves a half-written or failed build, and
// it picks the new one up by itself (it re-reads index.html when the file changes):
// refresh the browser after "build/ updated". `vite build --watch` is not used: with
// SvelteKit it can stamp the page and its chunks with different versions, and
// rolldown's watch mode fails after a few rebuilds.
import { existsSync, renameSync, rmSync, watch } from 'node:fs';
import { spawn } from 'node:child_process';

const OUT = 'build';
const NEXT = '.build-next';
const PREV = '.build-prev';
const WATCHED = [
	'src',
	'static',
	'messages',
	'project.inlang',
	'svelte.config.js',
	'vite.config.ts'
];
// Written by the build itself (paraglide compiles messages into it): never a trigger.
const IGNORED = [/^lib[/\\]paraglide([/\\]|$)/, /~$/, /\.sw[px]$/];

let building = null;
let again = false;
let timer = null;

function build() {
	if (building) {
		again = true;
		return;
	}
	const started = Date.now();
	console.log('[build:watch] building…');
	rmSync(NEXT, { recursive: true, force: true });
	building = spawn(
		process.execPath,
		['node_modules/vite/bin/vite.js', 'build', '--logLevel', 'warn'],
		{
			stdio: 'inherit',
			env: { ...process.env, UI_BUILD_OUT: NEXT }
		}
	);
	building.on('exit', (code) => {
		building = null;
		const seconds = ((Date.now() - started) / 1000).toFixed(1);
		if (code === 0 && existsSync(`${NEXT}/index.html`)) {
			rmSync(PREV, { recursive: true, force: true });
			if (existsSync(OUT)) renameSync(OUT, PREV);
			renameSync(NEXT, OUT);
			rmSync(PREV, { recursive: true, force: true });
			console.log(`[build:watch] build/ updated in ${seconds}s, refresh the browser`);
		} else {
			console.log(`[build:watch] build failed after ${seconds}s; build/ left unchanged`);
		}
		if (again) {
			again = false;
			build();
		}
	});
}

function changed(filename) {
	if (filename && IGNORED.some((pattern) => pattern.test(filename))) return;
	clearTimeout(timer);
	timer = setTimeout(build, 300);
}

for (const path of WATCHED) {
	if (existsSync(path)) watch(path, { recursive: true }, (_event, filename) => changed(filename));
}
console.log(`[build:watch] watching ${WATCHED.join(', ')}`);
build();
