import type { NextRequest } from 'next/server';
import { gatekeeper } from '@/lib/gatekeeper';

export function middleware(request: NextRequest) {
  return gatekeeper.middleware(request);
}

export const config = {
  matcher: ['/((?!_next/static|_next/image|favicon.ico).*)']
};
