<script lang="ts">
	let {
		class: className = "",
		size = 24,
		...rest
	}: { class?: string; size?: number; [key: string]: any } = $props();
</script>

<svg
	width={size}
	height={size}
	viewBox={size <= 16 ? "0 0 16 16" : "0 0 24 24"}
	fill="none"
	stroke="currentColor"
	stroke-width="2"
	stroke-linecap="round"
	stroke-linejoin="round"
	class={className}
	{...rest}
>
	<path d={size <= 16 ? "M4 6l4 4 4-4" : "M6 9l6 6 6-6"} />
</svg>
