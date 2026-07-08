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
	<path d="M14 4.5v7a1.5 1.5 0 0 1-1.5 1.5h-9A1.5 1.5 0 0 1 2 11.5v-7" />
	<path d="M2 4.5A1.5 1.5 0 0 1 3.5 3h9A1.5 1.5 0 0 1 14 4.5" />
	<path d="M8 8v3" />
	<path d="M5.5 9.5L8 8l2.5 1.5" />
</svg>
