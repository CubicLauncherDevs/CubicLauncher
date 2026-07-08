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
	<rect x="1" y="2" width="14" height="12" rx="2" />
	<circle cx="5" cy="6" r="1.5" />
	<path d="M1 12l3-3 2 2 4-4 5 5" />
</svg>
