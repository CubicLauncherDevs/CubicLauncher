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
	<rect x="1" y="1" width="6" height="6" />
	<rect x="9" y="1" width="6" height="6" />
	<rect x="9" y="9" width="6" height="6" />
	<rect x="1" y="9" width="6" height="6" />
</svg>
