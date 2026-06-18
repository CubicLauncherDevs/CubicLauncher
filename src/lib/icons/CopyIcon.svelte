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
	stroke-width="2"
	stroke-linecap="round"
	stroke-linejoin="round"
	class={className}
	{...rest}
>
	<rect x="9" y="9" width="13" height="13" rx="2" ry="2" />
	<path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" />
</svg>
