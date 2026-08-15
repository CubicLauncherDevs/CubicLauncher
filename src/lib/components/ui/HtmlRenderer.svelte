<script lang="ts">
	import { sanitizeHtml } from "$lib/util/markdown";

	interface Props {
		source: string;
		class?: string;
		onLinkClick?: (href: string) => void;
	}

	let {
		source,
		class: className = "markdown-body",
		onLinkClick,
	}: Props = $props();

	const html = $derived(sanitizeHtml(source));

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
