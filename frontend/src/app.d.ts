// See https://svelte.dev/docs/kit/types#app.d.ts
declare global {
  namespace App {
    interface Locals {
      /** Decrypted BFF session, or null when signed out. Tokens never leave the server. */
      session: {
        profile: {
          sub?: string;
          email?: string;
          emailVerified?: boolean;
          name?: string;
          givenName?: string;
          familyName?: string;
          picture?: string;
        } | null;
        accessToken: string;
        idToken?: string;
        refreshToken?: string;
        scope?: string;
        expiresAt: number;
      } | null;
    }
  }
}

export {};
