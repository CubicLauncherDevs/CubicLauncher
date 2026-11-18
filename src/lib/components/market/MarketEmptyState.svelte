<script lang="ts">
	import Icon from "#lib/icons/Icon.svelte";

	interface Props {
		title: string;
		subtitle?: string;
	}

	let { title, subtitle }: Props = $props();
</script>

<div class="market-empty">
	<div class="market-empty-icon">
		<Icon name="instance:puzzle" size={28} />
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
		padding: 32px 24px;
		gap: 12px;
		color: var(--text-secondary);
		text-align: center;
	}

	.market-empty-icon {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 64px;
		height: 64px;
		border: 1px solid var(--border);
		border-radius: var(--border-radius);
		background: var(--surface-selected);
	}

	.market-empty-title {
		font-size: 1.05rem;
		font-weight: 700;
		color: var(--text-primary);
		margin: 0;
	}

	.market-empty-subtitle {
		font-size: 0.8rem;
		margin: 0;
		color: var(--text-tertiary, var(--text-secondary));
		max-width: 360px;
		line-height: 1.6;
	}
</style>
