<script lang="ts">
	import type { SVGAttributes } from "svelte/elements";

	let {
		class: className = "",
		size = 16,
		...rest
	}: {
		class?: string;
		size?: number;
	} & SVGAttributes<SVGSVGElement> = $props();
</script>

<svg
	width={size}
	height={size}
	viewBox="0 0 16 16"
	fill="none"
	stroke="currentColor"
	stroke-width="1.5"
	stroke-linecap="round"
	stroke-linejoin="round"
	class={className}
	{...rest}
>
	<polygon points="8 1 10 5.5 14 6 11 9 12 14 8 11.5 4 14 5 9 2 6 6 5.5 8 1" />
</svg>
