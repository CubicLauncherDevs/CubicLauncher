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
</script>

<span
	class={"icon-svg " + className}
	style="width: {width}; height: {height}; background: currentColor; mask-image: url(/images/icons/ui/search.svg); -webkit-mask-image: url(/images/icons/ui/search.svg); mask-size: contain; -webkit-mask-size: contain; mask-repeat: no-repeat; -webkit-mask-repeat: no-repeat; mask-position: center; -webkit-mask-position: center;"
	aria-hidden="true"
	{...rest}
></span>
