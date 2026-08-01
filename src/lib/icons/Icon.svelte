<script lang="ts">
	import type { SvelteHTMLElements } from "svelte/elements";

	type SpanProps = SvelteHTMLElements["span"];

	let {
		class: className = "",
		src,
		size = 16,
		...rest
	}: {
		class?: string;
		src: string;
		size?: number;
	} & SpanProps = $props();
</script>

<span
	class={"icon-svg " + className}
	style="width: {size}px; height: {size}px; background: currentColor; mask-image: url({src}); -webkit-mask-image: url({src}); mask-size: contain; -webkit-mask-size: contain; mask-repeat: no-repeat; -webkit-mask-repeat: no-repeat; mask-position: center; -webkit-mask-position: center;"
	aria-hidden="true"
	{...rest}
></span>
