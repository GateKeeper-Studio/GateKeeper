import { Carta } from 'carta-md';
import DOMPurify from 'isomorphic-dompurify';

/**
 * Shared Carta instance used by both the dashboard markdown editor and the
 * public legal pages. The content is admin-authored markdown rendered to HTML,
 * so it is sanitized with DOMPurify to prevent stored XSS — important for an IdP.
 *
 * `isomorphic-dompurify` works in both SSR and the browser.
 */
export const carta = new Carta({
	sanitizer: (html: string) => DOMPurify.sanitize(html)
});
