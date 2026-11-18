<script lang="ts">
	import Icon from "#lib/icons/Icon.svelte";

	interface Props {
		className?: string;
	}

	let { className = "" }: Props = $props();
</script>

<span class="cubic-logo {className}" role="img" aria-label="Cubic Launcher">
	<Icon name="brand:cubic" width="100%" height="100%" />
</span>

<style>
	.cubic-logo {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: 100%;
		height: 100%;
		padding: 6px;
		color: var(--text-tertiary, var(--text-secondary));
	}
</style>
