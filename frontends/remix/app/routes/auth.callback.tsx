import type { LoaderFunctionArgs } from '@remix-run/node';
import { gatekeeper } from '~/gatekeeper.server';

export const loader = ({ request }: LoaderFunctionArgs) => gatekeeper.callbackLoader(request);
