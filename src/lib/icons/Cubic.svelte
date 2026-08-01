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
</script>

<span
	class="icon-svg"
	style="width: {width}; height: {height}; display: inline-block; background: currentColor; mask-image: url(/images/cubic.svg); -webkit-mask-image: url(/images/cubic.svg); mask-size: contain; -webkit-mask-size: contain; mask-repeat: no-repeat; -webkit-mask-repeat: no-repeat; mask-position: center; -webkit-mask-position: center;"
	aria-hidden="true"
	{...rest}
></span>
