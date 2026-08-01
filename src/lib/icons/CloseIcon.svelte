<script lang="ts">
	import type { SvelteHTMLElements } from "svelte/elements";

	type SpanProps = SvelteHTMLElements["span"];

	let {
		class: className = "",
		size = 24,
		...rest
	}: {
		class?: string;
		size?: number;
	} & SpanProps = $props();
</script>

<span
	class={"icon-svg " + className}
	style="width: {size}px; height: {size}px; background: currentColor; mask-image: url(/images/icons/ui/close.svg); -webkit-mask-image: url(/images/icons/ui/close.svg); mask-size: contain; -webkit-mask-size: contain; mask-repeat: no-repeat; -webkit-mask-repeat: no-repeat; mask-position: center; -webkit-mask-position: center;"
	aria-hidden="true"
	{...rest}
></span>
