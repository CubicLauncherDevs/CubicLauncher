<script lang="ts">
	import type { SvelteHTMLElements } from "svelte/elements";

	type SpanProps = SvelteHTMLElements["span"];

	let {
		width = "100%",
		height = "100%",
		...rest
	}: {
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
	class="icon-svg"
	style="width: {w}; height: {h}; display: inline-block; background: currentColor; mask-image: url(/images/cubic.svg); -webkit-mask-image: url(/images/cubic.svg); mask-size: contain; -webkit-mask-size: contain; mask-repeat: no-repeat; -webkit-mask-repeat: no-repeat; mask-position: center; -webkit-mask-position: center;"
	aria-hidden="true"
	{...rest}
></span>
