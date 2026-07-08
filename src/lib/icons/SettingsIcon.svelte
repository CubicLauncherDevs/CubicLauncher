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
	<circle cx="8" cy="8" r="2.5" />
	<path
		d="M8 1v2M8 13v2M2.5 8H1M15 8h-1.5M3.5 3.5L5 5M11 11l1.5 1.5M12.5 3.5L11 5M5 11L3.5 12.5"
	/>
</svg>
