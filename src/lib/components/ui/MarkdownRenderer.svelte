<script lang="ts">
	import { renderMarkdown } from "$lib/utils/markdown";

	interface Props {
		source: string;
		baseUrl?: string;
		class?: string;
		onLinkClick?: (href: string) => void;
	}

	let {
		source,
		baseUrl,
		class: className = "markdown-body",
		onLinkClick,
	}: Props = $props();

	const html = $derived(renderMarkdown(source, { baseUrl }));

	function handleClick(e: MouseEvent) {
		const a = (e.target as HTMLElement | null)?.closest("a");
		if (!a) return;

		const href = a.getAttribute("href");
		if (!href) return;

		if (href.startsWith("#")) {
			e.preventDefault();
			return;
		}

		if (!onLinkClick) return;

		e.preventDefault();
		onLinkClick(href);
	}
</script>

{#if html}
	<div class={className} role="presentation" onclick={handleClick}>
		{@html html}
	</div>
{:else}
	<div class={className} aria-hidden="true"></div>
{/if}

<style>
	:global(.markdown-body) {
		font-size: 0.92em;
		line-height: 1.6;
		color: var(--text-secondary);
		word-break: break-word;
	}

	:global(.markdown-body h1),
	:global(.markdown-body h2),
	:global(.markdown-body h3),
	:global(.markdown-body h4),
	:global(.markdown-body h5),
	:global(.markdown-body h6) {
		color: var(--text-primary);
		margin: 1.2em 0 0.6em;
		line-height: 1.3;
	}

	:global(.markdown-body h1) {
		font-size: 1.25em;
	}
	:global(.markdown-body h2) {
		font-size: 1.15em;
	}
	:global(.markdown-body h3) {
		font-size: 1.05em;
	}
	:global(.markdown-body h4),
	:global(.markdown-body h5),
	:global(.markdown-body h6) {
		font-size: 1em;
	}

	:global(.markdown-body p) {
		margin: 0.6em 0;
	}

	:global(.markdown-body a) {
		color: var(--accent);
		text-decoration: none;
	}

	:global(.markdown-body a:hover) {
		text-decoration: underline;
	}

	:global(.markdown-body img) {
		max-width: 100%;
		height: auto;
		border-radius: var(--border-radius-sm);
	}

	:global(.markdown-body code) {
		background: rgba(255, 255, 255, 0.06);
		padding: 0.15em 0.35em;
		border-radius: 4px;
		font-family: monospace;
		font-size: 0.9em;
	}

	:global(.markdown-body pre) {
		background: rgba(255, 255, 255, 0.04);
		padding: 10px;
		border-radius: var(--border-radius-sm);
		overflow-x: auto;
		border: 1px solid var(--border);
	}

	:global(.markdown-body pre code) {
		background: none;
		padding: 0;
		border-radius: 0;
		font-size: 0.92em;
	}

	:global(.markdown-body ul),
	:global(.markdown-body ol) {
		padding-left: 1.4em;
		margin: 0.6em 0;
	}

	:global(.markdown-body table) {
		width: 100%;
		border-collapse: collapse;
		font-size: 0.92em;
	}

	:global(.markdown-body th),
	:global(.markdown-body td) {
		border: 1px solid var(--border);
		padding: 6px 8px;
		text-align: left;
	}

	:global(.markdown-body th) {
		background: rgba(255, 255, 255, 0.04);
	}

	:global(.markdown-body blockquote) {
		border-left: 3px solid var(--accent);
		padding-left: 10px;
		margin: 0.8em 0;
		color: var(--text-secondary);
	}

	:global(.markdown-body hr) {
		border: none;
		border-top: 1px solid var(--border);
		margin: 1em 0;
	}
</style>
