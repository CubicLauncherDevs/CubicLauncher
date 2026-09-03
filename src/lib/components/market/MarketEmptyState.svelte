<script lang="ts">
	import Icon from "$lib/icons/Icon.svelte";

	interface Props {
		title: string;
		subtitle?: string;
	}

	let { title, subtitle }: Props = $props();
</script>

<div class="market-empty">
	<div class="market-empty-icon">
		<Icon name="instance:box" size={40} />
	</div>
	<p class="market-empty-title">{title}</p>
	{#if subtitle}
		<p class="market-empty-subtitle">{subtitle}</p>
	{/if}
</div>

<style>
	.market-empty {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		height: 100%;
		min-height: 240px;
		padding: 40px;
		gap: 12px;
		color: var(--text-secondary);
		text-align: center;
	}

	.market-empty-icon {
		display: flex;
		align-items: center;
		justify-content: center;
		opacity: 0.7;
	}

	.market-empty-title {
		font-size: 0.95rem;
		font-weight: 700;
		margin: 0;
		text-transform: uppercase;
		letter-spacing: 1px;
	}

	.market-empty-subtitle {
		font-size: 0.8rem;
		margin: 0;
		opacity: 0.7;
		max-width: 320px;
		line-height: 1.4;
	}
</style>
