import type { NextRequest } from 'next/server';
import { gatekeeper } from '@/lib/gatekeeper';

export const POST = (request: NextRequest) => gatekeeper.signOutHandler(request);
