<script lang="ts">
	let {
		class: className = "",
		size = 16,
		...rest
	}: { class?: string; size?: number; [key: string]: any } = $props();
</script>

<svg
	width={size}
	height={size}
	viewBox="0 0 24 24"
	fill="none"
	stroke="currentColor"
	stroke-width="2.5"
	stroke-linecap="round"
	stroke-linejoin="round"
	class={className}
	{...rest}
>
	<polyline points="20 6 9 17 4 12" />
</svg>
