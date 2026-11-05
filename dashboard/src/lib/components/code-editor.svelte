<script lang="ts">
	import { EditorView, keymap, lineNumbers, highlightActiveLine } from '@codemirror/view';
	import { EditorState, Compartment } from '@codemirror/state';
	import {
		defaultKeymap,
		history,
		historyKeymap,
		indentWithTab
	} from '@codemirror/commands';
	import {
		bracketMatching,
		defaultHighlightStyle,
		indentOnInput,
		syntaxHighlighting
	} from '@codemirror/language';
	import { html } from '@codemirror/lang-html';
	import { oneDark } from '@codemirror/theme-one-dark';
	import { mode } from 'mode-watcher';

	// A lightweight CodeMirror 6 editor with HTML/MJML syntax highlighting, mounted
	// directly (no wrapper lib) so it stays compatible with Svelte 5. `value` is a
	// two-way bindable string; `insertAtCursor` is exported for "insert variable"
	// buttons.
	let {
		value = $bindable(''),
		class: className = ''
	}: { value?: string; class?: string } = $props();

	let host = $state<HTMLDivElement | null>(null);
	let view: EditorView | null = null;
	const themeCompartment = new Compartment();

	// oneDark bundles its own highlight style; in light mode we pair the default
	// CodeMirror theme with the default highlight style.
	function themeExtension(dark: boolean) {
		return dark
			? oneDark
			: syntaxHighlighting(defaultHighlightStyle, { fallback: true });
	}

	const baseTheme = EditorView.theme({
		'&': { fontSize: '12px', height: '100%', backgroundColor: 'transparent' },
		'.cm-content': {
			fontFamily: 'ui-monospace, SFMono-Regular, Menlo, Consolas, monospace'
		},
		'.cm-scroller': { overflow: 'auto' },
		'&.cm-focused': { outline: 'none' },
		'.cm-gutters': { backgroundColor: 'transparent', border: 'none' }
	});

	$effect(() => {
		if (!host || view) return;

		view = new EditorView({
			parent: host,
			state: EditorState.create({
				doc: value,
				extensions: [
					lineNumbers(),
					history(),
					indentOnInput(),
					bracketMatching(),
					highlightActiveLine(),
					keymap.of([...defaultKeymap, ...historyKeymap, indentWithTab]),
					html(),
					EditorView.lineWrapping,
					baseTheme,
					themeCompartment.of(themeExtension(mode.current === 'dark')),
					EditorView.updateListener.of((update) => {
						if (update.docChanged) {
							value = update.state.doc.toString();
						}
					})
				]
			})
		});

		return () => {
			view?.destroy();
			view = null;
		};
	});

	// External updates to `value` (e.g. reset-to-default or switching situations)
	// are pushed into the editor, guarding against the change-listener feedback loop.
	$effect(() => {
		const next = value;
		if (view && view.state.doc.toString() !== next) {
			view.dispatch({
				changes: { from: 0, to: view.state.doc.length, insert: next }
			});
		}
	});

	// Re-theme when the app's light/dark mode toggles.
	$effect(() => {
		const dark = mode.current === 'dark';
		view?.dispatch({
			effects: themeCompartment.reconfigure(themeExtension(dark))
		});
	});

	// Inserts text at the caret (replacing any selection) and refocuses — used by
	// the variable-chip buttons.
	export function insertAtCursor(text: string) {
		if (!view) {
			value += text;
			return;
		}
		const { from, to } = view.state.selection.main;
		view.dispatch({
			changes: { from, to, insert: text },
			selection: { anchor: from + text.length }
		});
		view.focus();
	}
</script>

<div bind:this={host} class={className}></div>
