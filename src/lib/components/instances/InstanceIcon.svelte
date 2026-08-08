<script lang="ts">
	import { getDisplayIconSrc } from "$lib/icons/logos";
	import type { SvelteHTMLElements } from "svelte/elements";

	interface Props {
		icon: string | null;
		alt: string;
		class?: string;
		size?: string | number;
	}

	let {
		icon,
		alt,
		class: className = "",
		size,
		...rest
	}: Props & SvelteHTMLElements["span"] = $props();

	const style = $derived(
		size
			? `width: ${typeof size === "number" ? `${size}px` : size}; height: ${typeof size === "number" ? `${size}px` : size};`
			: "",
	);
</script>

{#if icon}
	<img
		src={getDisplayIconSrc(icon)}
		{alt}
		class="instance-icon-img {className}"
	/>
{:else}
	<span
		class="instance-icon-fallback {className}"
		aria-label={alt}
		{style}
		{...rest}
	></span>
{/if}

<style>
	.instance-icon-img,
	.instance-icon-fallback {
		width: 100%;
		height: 100%;
		display: block;
	}

	.instance-icon-img {
		object-fit: contain;
	}

	.instance-icon-fallback {
		background: var(--cubic-logo);
		background-size: contain;
		background-repeat: no-repeat;
		background-position: center;
	}
</style>
