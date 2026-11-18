<script lang="ts">
	let {
		class: className = "",
		size = 32,
	}: {
		class?: string;
		size?: number;
	} = $props();

	const diameter = $derived(typeof size === "number" ? `${size}px` : size);
</script>

<div
	class="ld-ring {className}"
	style:--loading-spinner-size={diameter}
	aria-hidden="true"
></div>

<style>
	.ld-ring {
		width: var(--loading-spinner-size);
		height: var(--loading-spinner-size);
		box-sizing: border-box;
		display: inline-block;
		flex: 0 0 auto;
		border: 3px solid
			color-mix(in srgb, var(--ink, currentColor) 18%, transparent);
		border-top-color: var(--ink, currentColor);
		border-radius: 50%;
		animation: spin 0.9s linear infinite;
	}

	@keyframes spin {
		to {
			transform: rotate(360deg);
		}
	}

	@media (prefers-reduced-motion: reduce) {
		.ld-ring {
			animation: none;
		}
	}
</style>
