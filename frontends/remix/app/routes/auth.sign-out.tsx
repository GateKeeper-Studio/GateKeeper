import type { ActionFunctionArgs } from '@remix-run/node';
import { gatekeeper } from '~/gatekeeper.server';

export const action = ({ request }: ActionFunctionArgs) => gatekeeper.signOutAction(request);
