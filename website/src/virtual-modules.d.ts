// Ambient declarations for Vite virtual modules.
//
// This file must NOT contain a top-level `import` or `export` — that would make it a module,
// and `declare module` inside a module is treated as an *augmentation* of an existing module
// rather than a new ambient one. (That is why this can't live in app.d.ts, which ends in
// `export {}`.)

// Built at compile time by docs-tooling/search-index-plugin.js.
declare module 'virtual:docs-search-index' {
	const index: import('$lib/docs/types').DocsSearchEntry[];
	export default index;
}
