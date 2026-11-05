// See https://svelte.dev/docs/kit/types#app.d.ts
import type { GateKeeperSession } from '@gatekeeper/sveltekit-bff';

declare global {
  namespace App {
    interface Locals {
      /** Decrypted BFF session, or null when signed out. Tokens never leave the server. */
      session: GateKeeperSession | null;
    }
  }
}

export {};
