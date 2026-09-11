<script lang="ts">
	let { size = 18 }: { size?: number } = $props();
</script>

<svg
	width={size}
	height={size}
	viewBox="0 0 24 24"
	fill="none"
	stroke="currentColor"
	stroke-width="1.6"
	stroke-linecap="round"
	stroke-linejoin="round"
	aria-hidden="true"
>
	<circle cx="12" cy="12" r="9" />
	<path d="M3 12h18M12 3c5 5 5 13 0 18-5-5-5-13 0-18Z" />
</svg>
