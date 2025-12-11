"use client";

import { signOut } from "@/lib/utils/sign-out";

export function LogoutButton() {
  return (
    <button
      onClick={() => signOut()}
      type="button"
      className="px-4 py-2 text-white bg-blue-500 rounded-md shadow-md hover:bg-blue-600 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 focus:ring-offset-white"
    >
      Sign out
    </button>
  );
}
