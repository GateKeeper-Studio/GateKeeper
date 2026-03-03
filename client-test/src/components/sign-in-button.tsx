"use client";

import { signIn } from "@/lib/utils/sign-in";

export function SignInButton() {
  return (
    <button
      onClick={() =>
        signIn({ redirectUri: "http://localhost:3001/api/callback" })
      }
      type="button"
      className="px-4 py-2 text-white bg-blue-500 rounded-md shadow-md hover:bg-blue-600 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 focus:ring-offset-white"
    >
      Sign In With GateKeeper
    </button>
  );
}
