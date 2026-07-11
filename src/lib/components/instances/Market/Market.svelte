<script lang="ts">
	import { onDestroy } from "svelte";
	import { t } from "$lib/i18n";
	import type { InstanceDto } from "$lib/types/types";
	import { createMarketState } from "$lib/state/marketState.svelte";
	import type { ContentType } from "$lib/types/market";
	import MarketFilterPanel from "$lib/components/market/MarketFilterPanel.svelte";
	import MarketItem from "$lib/components/market/MarketItem.svelte";
	import MarketDetail from "$lib/components/market/MarketDetail.svelte";
	import MarketEmptyState from "$lib/components/market/MarketEmptyState.svelte";
	import MarketLayout from "$lib/components/market/MarketLayout.svelte";

	interface Props {
		instance: InstanceDto;
		contentType?: ContentType;
	}

	let { instance, contentType = "mods" }: Props = $props();

	function init() {
		return createMarketState(instance, contentType);
	}
	const state = init();

	onDestroy(() => {
		try {
			state.destroy();
		} catch (e) {
			console.error("[Market] destroy error:", e);
		}
	});

	const emptyState = $derived.by(() => {
		if (state.filters.source === "local") {
			return {
				title: t("market.empty.localTitle"),
				subtitle: t("market.empty.localSubtitle"),
			};
		}
		return {
			title: state.filters.query
				? t("market.empty.searchTitle")
				: t("market.empty.marketTitle"),
			subtitle: state.filters.query
				? t("market.empty.searchSubtitle")
				: t("market.empty.marketSubtitle"),
		};
	});
</script>

<div class="market-root">
	<MarketLayout
		items={state.items}
		selectedId={state.selectedId}
		loading={state.loading}
		loadingMore={state.loadingMore}
		hasMore={state.hasMore}
		error={state.error}
		onSelect={state.selectProject}
		onLoadMore={state.loadMore}
		keyFn={(p) => p.id}
	>
		{#snippet filterPanel()}
			<MarketFilterPanel
				filters={state.filters}
				{contentType}
				loading={state.loading}
				onSourceChange={state.setSource}
				onQueryChange={state.setQuery}
				onSortChange={state.setSort}
				onCategoryChange={state.setCategory}
				onLocalSortChange={state.setLocalSort}
				onRefresh={state.refresh}
			/>
		{/snippet}

		{#snippet itemSnippet(project)}
			{#if Object.keys(project).length === 0}
				<MarketEmptyState
					title={emptyState.title}
					subtitle={emptyState.subtitle}
				/>
			{:else}
				<MarketItem
					{project}
					selected={project.id === state.selectedId}
					onSelect={() => state.selectProject(project.id)}
					onInstall={state.filters.source !== "local" &&
					!project.installed
						? () => {
								const version = state.selectedVersion();
								if (version) state.install(project, version);
							}
						: undefined}
				/>
			{/if}
		{/snippet}

		{#snippet detailSnippet()}
			{#if state.selectedProject}
				{@const project = state.selectedProject}
				<MarketDetail
					{project}
					{contentType}
					detail={state.detail}
					selectedVersion={state.selectedVersion()}
					isVersionCompatible={state.isVersionCompatible}
					onVersionSelect={state.setSelectedVersion}
					onInstall={() => {
						const version = state.selectedVersion();
						if (version) return state.install(project, version);
					}}
					onUninstall={() => state.uninstall(project)}
					onToggleEnabled={() => state.toggleEnabled(project)}
					onClose={() => state.selectProject(null)}
				/>
			{/if}
		{/snippet}
	</MarketLayout>
</div>

<style>
	.market-root {
		position: absolute;
		inset: 0;
		display: flex;
		flex-direction: column;
		overflow: hidden;
		background: var(--bg-main);
	}
</style>
