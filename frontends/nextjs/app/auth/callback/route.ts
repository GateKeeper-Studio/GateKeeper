import type { NextRequest } from 'next/server';
import { gatekeeper } from '@/lib/gatekeeper';

export const GET = (request: NextRequest) => gatekeeper.callbackHandler(request);
