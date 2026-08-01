<script lang="ts">
	import type { SvelteHTMLElements } from "svelte/elements";

	type SpanProps = SvelteHTMLElements["span"];

	let {
		class: className = "",
		width = "1em",
		height = "1em",
		...rest
	}: {
		class?: string;
		width?: string | number;
		height?: string | number;
	} & SpanProps = $props();

	const w = $derived(
		typeof width === "number"
			? `${width}px`
			: /^\d+(\.\d+)?$/.test(width)
				? `${width}px`
				: width,
	);
	const h = $derived(
		typeof height === "number"
			? `${height}px`
			: /^\d+(\.\d+)?$/.test(height)
				? `${height}px`
				: height,
	);
</script>

<span
	class={"icon-svg " + className}
	style="width: {w}; height: {h}; background: currentColor; mask-image: url(/images/icons/ui/trash.svg); -webkit-mask-image: url(/images/icons/ui/trash.svg); mask-size: contain; -webkit-mask-size: contain; mask-repeat: no-repeat; -webkit-mask-repeat: no-repeat; mask-position: center; -webkit-mask-position: center;"
	aria-hidden="true"
	{...rest}
></span>
