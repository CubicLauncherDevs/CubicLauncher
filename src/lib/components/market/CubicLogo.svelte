<script lang="ts">
	interface Props {
		className?: string;
	}

	let { className = "" }: Props = $props();
</script>

<img
	class="cubic-logo {className}"
	src="/images/cubic.svg"
	alt="Cubic Launcher"
/>

<style>
	.cubic-logo {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: 100%;
		height: 100%;
		object-fit: contain;
	}
</style>
