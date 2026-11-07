const KINDS = new Set(['note', 'tip', 'important', 'warning', 'caution']);

/**
 * GitHub-style alerts — `> [!WARNING]` followed by the body — become <Callout> components.
 *
 *     > [!WARNING]
 *     > Redirect URIs are matched exactly.
 *
 * Two details worth knowing:
 *
 *  - `[!WARNING]` is parsed by remark as a shortcut `linkReference` (there is no matching
 *    link definition), not as plain text. Depending on the surrounding content it can also
 *    survive as literal text, so both shapes are handled.
 *
 *  - The output is emitted as raw `html` siblings wrapping the blockquote's *existing*
 *    children, so the markdown inside the callout is still rendered as markdown. And it is
 *    namespaced `<Components.Callout>` on purpose: mdsvex only rewrites bare tag names on
 *    hast *element* nodes, never on raw HTML, so a plugin emitting `<Callout>` would compile
 *    to an undefined reference. (Same reason the Shiki highlighter emits `<Components.Mermaid>`.)
 */
export function remarkCallouts() {
	return (/** @type {any} */ tree) => {
		const root = tree.children ?? [];

		// Walk backwards: each match splices extra siblings in, which would shift a forward index.
		for (let i = root.length - 1; i >= 0; i--) {
			const node = root[i];
			if (node.type !== 'blockquote') continue;

			const para = node.children?.[0];
			if (!para || para.type !== 'paragraph') continue;

			const head = para.children?.[0];
			let kind;

			if (
				head?.type === 'linkReference' &&
				typeof head.label === 'string' &&
				head.label.startsWith('!')
			) {
				kind = head.label.slice(1).toLowerCase();
				para.children.shift();
			} else if (head?.type === 'text') {
				const match = /^\[!(\w+)\]\s*/.exec(head.value);
				if (!match) continue;
				kind = match[1].toLowerCase();
				head.value = head.value.slice(match[0].length);
			} else {
				continue;
			}

			if (!KINDS.has(kind)) continue;

			// Drop the newline left behind by `> [!NOTE]\n> body`, and the paragraph itself if
			// the marker was all it contained.
			const first = para.children[0];
			if (first?.type === 'text') first.value = first.value.replace(/^\s*\n?/, '');
			if (para.children.length === 0) node.children.shift();

			root.splice(
				i,
				1,
				{ type: 'html', value: `<Components.Callout type="${kind}">` },
				...node.children,
				{ type: 'html', value: '</Components.Callout>' }
			);
		}
	};
}
