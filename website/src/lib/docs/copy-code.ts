import { toast } from 'svelte-sonner';

// Kept as one unbroken literal so Tailwind's source scan picks the classes up.
const BUTTON_CLASS =
	'absolute right-2 top-2 inline-flex size-8 items-center justify-center rounded-md border bg-background/80 text-muted-foreground opacity-0 backdrop-blur transition hover:text-foreground focus-visible:opacity-100 group-hover/code:opacity-100';

const COPY_ICON = `<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect width="14" height="14" x="8" y="8" rx="2" ry="2"/><path d="M4 16c-1.1 0-2-.9-2-2V4c0-1.1.9-2 2-2h10c1.1 0 2 .9 2 2"/></svg>`;

/**
 * Adds a copy button to every `.docs-code` block emitted by the Shiki highlighter.
 *
 * Actions only run in the browser, so this never executes during prerender. The code is read
 * back from `pre.textContent`: the DOM has already decoded the brace entities the highlighter
 * escaped, so what we copy is byte-for-byte the original source.
 */
export function copyCode(node: HTMLElement) {
	const teardown: Array<() => void> = [];

	for (const block of node.querySelectorAll<HTMLElement>('.docs-code')) {
		block.classList.add('group/code');

		const button = document.createElement('button');
		button.type = 'button';
		button.className = BUTTON_CLASS;
		button.setAttribute('aria-label', 'Copy code');
		button.innerHTML = COPY_ICON;

		const onClick = async () => {
			const code = block.querySelector('pre')?.textContent ?? '';
			try {
				await navigator.clipboard.writeText(code);
				toast.success('Copied to clipboard');
			} catch {
				toast.error('Could not copy to clipboard');
			}
		};

		button.addEventListener('click', onClick);
		block.appendChild(button);
		teardown.push(() => button.removeEventListener('click', onClick));
	}

	return {
		destroy: () => teardown.forEach((fn) => fn())
	};
}
