<script lang="ts">
	import { onDestroy, type Snippet } from "svelte";
	import type { MarketProject } from "$lib/types/market";
	import MarketGrid from "./MarketGrid.svelte";
	import VirtualList from "$lib/components/layout/VirtualList.svelte";
	import { t } from "$lib/i18n";

	interface Props {
		items: MarketProject[];
		itemCount: number;
		getItem: (index: number) => MarketProject | null | undefined;
		onRangeNeeded: (first: number, last: number) => void;
		total: number;
		resultsRevision: number;
		selectedId: string | null;
		detailTitle: string;
		loading: boolean;
		loadingMore: boolean;
		hasMore: boolean;
		error: string | null;
		onClose: () => void;
		onRetry: () => void;
		onLoadMore: () => void;
		filterPanel: Snippet;
		itemSnippet: Snippet<[MarketProject]>;
		emptySnippet: Snippet;
		detailSnippet: Snippet;
		listView?: boolean;
	}

	let {
		items,
		itemCount,
		getItem,
		onRangeNeeded,
		total,
		resultsRevision,
		selectedId,
		detailTitle,
		loading,
		loadingMore,
		hasMore,
		error,
		onClose,
		onRetry,
		onLoadMore,
		filterPanel,
		itemSnippet,
		emptySnippet,
		detailSnippet,
		listView = false,
	}: Props = $props();
	let lastCatalogFocus: WeakRef<HTMLElement> | undefined;
	onDestroy(() => {
		lastCatalogFocus = undefined;
	});

	function focusDetail(node: HTMLElement) {
		const previous = lastCatalogFocus;
		queueMicrotask(() => {
			if (node.isConnected)
				(
					node.querySelector<HTMLButtonElement>("button") ?? node
				).focus();
		});
		return {
			destroy() {
				queueMicrotask(() => {
					const target = previous?.deref();
					if (target?.isConnected)
						target.focus({ preventScroll: true });
				});
			},
		};
	}

	function handleDetailKey(event: KeyboardEvent) {
		if (!selectedId || event.defaultPrevented || event.key !== "Escape")
			return;
		if (
			event.target instanceof Element &&
			event.target.closest('[role="dialog"], dialog')
		)
			return;
		event.preventDefault();
		onClose();
	}
</script>

<svelte:window onkeydown={handleDetailKey} />

<div class="market-layout">
	<div
		class="market-catalog"
		class:covered={selectedId !== null}
		inert={selectedId !== null}
		onfocusin={(event) => {
			if (event.target instanceof HTMLElement)
				lastCatalogFocus = new WeakRef(event.target);
		}}
	>
		{@render filterPanel()}
		<div class="results-bar" role="status" aria-live="polite">
			{#if loading}
				<span class="spinner"></span>
				<span>{t("market.browse.searching")}</span>
			{:else if !error}
				<span>{t("market.browse.results", { count: total })}</span>
			{/if}
		</div>
		<div class="market-results" aria-busy={loading} inert={loading}>
			{#if items.length > 0}
				{#if listView}
					<VirtualList
						{items}
						{onRangeNeeded}
						active={!selectedId && !loading}
						resetKey={resultsRevision}
						itemHeight={84}
						itemHeightVar="--market-installed-row-height"
						itemGap={{
							variable: "--market-installed-row-gap",
							fallback: 6,
						}}
						padding={8}
						hideScrollbar={false}
						keyFn={(item) => item.id}
					>
						{#snippet children(project)}
							{@render itemSnippet(project)}
						{/snippet}
					</VirtualList>
				{:else}
					{#key resultsRevision}
						<MarketGrid
							{items}
							count={itemCount}
							{getItem}
							{onRangeNeeded}
							{onLoadMore}
							busy={loading || loadingMore || !!error}
							active={!selectedId}
						>
							{#snippet children(project)}
								{@render itemSnippet(project)}
							{/snippet}
						</MarketGrid>
					{/key}
				{/if}
			{:else if loading}
				<MarketGrid
					count={12}
					getItem={() => undefined}
					{onRangeNeeded}
					{onLoadMore}
					busy={true}
					active={false}
				>
					{#snippet children(project)}
						{@render itemSnippet(project)}
					{/snippet}
				</MarketGrid>
			{:else if !error}
				<div class="results-empty">{@render emptySnippet()}</div>
			{/if}
		</div>
		{#if error}
			<div class="results-footer error" role="alert">
				<p>{error}</p>
				<button type="button" onclick={onRetry}
					>{t("market.browse.retry")}</button
				>
			</div>
		{:else if items.length > 0}
			<div class="results-footer">
				{#if loadingMore}
					<span class="spinner"></span><span role="status"
						>{t("market.browse.loadingMore")}</span
					>
				{:else if hasMore}
					<button
						type="button"
						disabled={loading}
						onclick={onLoadMore}
						>{t("market.browse.loadMore")}</button
					>
				{:else}
					<span>{t("market.browse.endResults")}</span>
				{/if}
			</div>
		{/if}
	</div>
	{#if selectedId}
		{#key selectedId}
			<section
				class="market-detail-pane"
				aria-label={detailTitle}
				tabindex="-1"
				use:focusDetail
			>
				{@render detailSnippet()}
			</section>
		{/key}
	{/if}
</div>

<style>
	.market-layout {
		container: market / inline-size;
		position: relative;
		height: 100%;
		min-height: 0;
		overflow: hidden;
		background: var(--bg-main);
	}
	.market-catalog {
		display: flex;
		flex-direction: column;
		height: 100%;
		min-height: 0;
	}
	.market-catalog.covered {
		visibility: hidden;
	}
	.results-bar {
		display: flex;
		align-items: center;
		gap: 8px;
		min-height: 40px;
		padding: 0 var(--market-page-padding);
		color: var(--text-secondary);
		font-size: 0.75rem;
		flex-shrink: 0;
	}
	.market-results {
		flex: 1;
		min-height: 0;
		margin: 0 var(--market-page-padding);
		overflow: hidden;
	}
	.results-empty {
		height: 100%;
		overflow-y: auto;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		color: var(--text-secondary);
	}
	.results-footer {
		display: flex;
		justify-content: center;
		align-items: center;
		flex-wrap: wrap;
		gap: 10px;
		padding: 10px var(--market-page-padding);
		min-height: 48px;
		box-sizing: border-box;
		font-size: 0.8rem;
		color: var(--text-secondary);
	}
	.results-footer.error {
		color: var(--color-error);
	}
	.results-footer p {
		margin: 0;
		overflow-wrap: anywhere;
		max-height: 80px;
		overflow-y: auto;
	}
	.results-footer button {
		border: var(--border-width) solid var(--border);
		border-radius: var(--border-radius-sm);
		background: var(--surface-card);
		color: var(--text-primary);
		padding: 7px 16px;
		cursor: pointer;
		font: inherit;
	}
	.results-footer button:hover:not(:disabled) {
		border-color: var(--accent);
		background: var(--surface-hover);
	}
	.results-footer button:disabled {
		opacity: 0.5;
		cursor: wait;
	}
	.results-footer button:focus-visible {
		outline: 2px solid var(--accent);
		outline-offset: 2px;
	}
	.market-detail-pane {
		position: absolute;
		inset: 0;
		background: var(--bg-main);
		overflow: hidden;
		outline: none;
	}
	.spinner {
		width: 14px;
		height: 14px;
		border: 2px solid var(--border);
		border-top-color: var(--accent);
		border-radius: 50%;
		flex-shrink: 0;
		animation: spin 0.8s linear infinite;
	}
	@keyframes spin {
		to {
			transform: rotate(360deg);
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.spinner {
			animation: none;
		}
	}
	@container market (max-width: 700px) {
		.market-results {
			margin: 0 var(--market-page-padding-compact);
		}
		.results-bar {
			padding: 0 var(--market-page-padding-compact);
		}
		.results-footer {
			padding-inline: var(--market-page-padding-compact);
		}
	}
</style>
