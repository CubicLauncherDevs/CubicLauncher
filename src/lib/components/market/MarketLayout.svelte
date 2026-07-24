<script lang="ts">
	import type { Snippet } from "svelte";
	import { fade } from "svelte/transition";
	import type { MarketProject } from "$lib/types/market";
	import VirtualList from "$lib/components/layout/VirtualList.svelte";
	import Lupa from "$lib/icons/Lupa.svelte";
	import { t } from "$lib/i18n";

	interface Props {
		items: MarketProject[];
		selectedId: string | null;
		loading: boolean;
		loadingMore: boolean;
		hasMore: boolean;
		error: string | null;
		onSelect: (id: string) => void;
		onLoadMore: () => void;
		filterPanel: Snippet;
		itemSnippet: Snippet<[MarketProject]>;
		detailSnippet: Snippet;
		keyFn?: (item: MarketProject) => string | number;
	}

	let {
		items,
		selectedId,
		loading,
		loadingMore,
		hasMore,
		error,
		onSelect,
		onLoadMore,
		filterPanel,
		itemSnippet,
		detailSnippet,
		keyFn,
	}: Props = $props();
</script>

<div class="market-layout">
	<div class="market-list-pane">
		{@render filterPanel()}

		<div class="market-list-content">
			{#if error}
				<div class="market-list-error">
					<span>⚠</span>
					<p>{error}</p>
				</div>
			{:else if items.length === 0 && !loading}
				<div class="market-list-empty">
					{@render itemSnippet({} as unknown as MarketProject)}
				</div>
			{:else}
				<VirtualList
					{items}
					itemHeight={106}
					{keyFn}
					onNearEnd={onLoadMore}
				>
					{#snippet children(project)}
						<!-- svelte-ignore a11y_click_events_have_key_events -->
						<!-- svelte-ignore a11y_no_static_element_interactions -->
						<div
							class="market-item-wrapper"
							class:selected={project.id === selectedId}
							onclick={() => onSelect(project.id)}
						>
							{@render itemSnippet(project)}
						</div>
					{/snippet}
				</VirtualList>

				{#if loadingMore}
					<div class="market-list-loading-more">
						<span class="spinner"></span>
						<span>Loading more...</span>
					</div>
				{:else if !hasMore && items.length > 0}
					<div class="market-list-end">
						— {items.length} results —
					</div>
				{/if}
			{/if}

			{#if loading && items.length === 0}
				<div class="market-list-loading">
					<span class="spinner"></span>
				</div>
			{/if}
		</div>
	</div>

	<div class="market-detail-pane" class:empty={!selectedId}>
		{#key selectedId}
			<div in:fade={{ duration: 150 }}>
				{#if selectedId}
					{@render detailSnippet()}
				{:else}
					<div class="market-empty">
						<span class="market-empty-icon">
							<Lupa width="32" height="32" />
						</span>
						<p class="market-empty-title">
							{t("market.detail.noSelection")}
						</p>
					</div>
				{/if}
			</div>
		{/key}
	</div>
</div>

<style>
	.market-layout {
		display: flex;
		height: 100%;
		overflow: hidden;
		background: var(--bg-main);
	}

	.market-list-pane {
		width: 25%;
		flex-shrink: 0;
		display: flex;
		flex-direction: column;
		min-width: 240px;
		overflow: hidden;
	}

	.market-list-content {
		flex: 1;
		overflow: hidden;
		padding: 12px 14px;
	}

	.market-item-wrapper {
		border-radius: var(--border-radius-sm);
		transition: background 0.15s;
		padding: 0 2px;
		cursor: pointer;
	}

	.market-item-wrapper:hover {
		background: rgba(255, 255, 255, 0.02);
	}

	.market-detail-pane {
		flex: 1;
		min-width: 0;
		background: var(--bg-sidebar);
		border-left: 1px solid var(--border);
		overflow-y: auto;
		overflow-x: hidden;
	}

	.market-detail-pane.empty .market-empty {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		height: 100%;
		padding: 40px;
		gap: 12px;
		color: var(--text-secondary);
		text-align: center;
	}

	.market-empty-icon {
		opacity: 0.7;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.market-empty-title {
		font-size: 0.95rem;
		font-weight: 700;
		margin: 0;
		text-transform: uppercase;
		letter-spacing: 1px;
	}

	.market-list-loading,
	.market-list-empty,
	.market-list-error {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		height: 100%;
		min-height: 240px;
		gap: 12px;
		color: var(--text-secondary);
		text-align: center;
		padding: 20px;
	}

	.market-list-error {
		color: var(--color-error);
	}

	.market-list-error p {
		margin: 0;
		font-size: 0.85rem;
		line-height: 1.4;
		max-width: 360px;
	}

	.market-list-loading-more,
	.market-list-end {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 10px;
		padding: 16px 0 8px;
		color: var(--text-secondary);
		font-size: 0.75rem;
	}

	.market-list-end {
		text-transform: uppercase;
		letter-spacing: 1px;
	}

	.spinner {
		width: 18px;
		height: 18px;
		border: 2px solid var(--border);
		border-top-color: var(--accent);
		border-radius: 50%;
		animation: spin 0.8s linear infinite;
	}

	@keyframes spin {
		to {
			transform: rotate(360deg);
		}
	}

	@media (max-width: 900px) {
		.market-layout {
			position: relative;
		}
		.market-list-pane {
			width: 100%;
			min-width: 0;
		}
		.market-detail-pane {
			position: absolute;
			inset: 0;
			z-index: 100;
			width: 100%;
			border-left: none;
		}
	}
</style>
