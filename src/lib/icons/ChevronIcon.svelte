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
	viewBox="0 0 16 16"
	fill="none"
	stroke="currentColor"
	stroke-width="2"
	stroke-linecap="round"
	stroke-linejoin="round"
	class={className}
	{...rest}
>
	<path d="M4 6l4 4 4-4" />
</svg>
