// See https://svelte.dev/docs/kit/types#app.d.ts
// for information about these interfaces

declare global {
	namespace App {
		// This app is a static SPA served by the identity provider itself (see
		// server/docs/SERVING-THE-DASHBOARD-FROM-THE-SERVER.md): there is no
		// server-side code, so no Locals.
		// interface Error {}
		// interface PageData {}
		// interface PageState {}
		// interface Platform {}
	}
}

export {};
