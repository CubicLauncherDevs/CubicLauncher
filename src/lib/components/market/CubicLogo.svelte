<script lang="ts">
	import cubicSvg from "@static/images/cubic.svg?raw";

	interface Props {
		className?: string;
	}

	let { className = "" }: Props = $props();
</script>

<span class="cubic-logo {className}">
	{@html cubicSvg}
</span>

<style>
	.cubic-logo {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		color: var(--text-secondary);
		width: 100%;
		height: 100%;
	}

	.cubic-logo :global(svg) {
		width: 100%;
		height: 100%;
	}
</style>
