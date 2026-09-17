<script lang="ts">
	import { onDestroy } from "svelte";
	import { ask } from "@tauri-apps/plugin-dialog";
	import { showErrorParsed } from "$lib/state/state.svelte";
	import { t } from "$lib/i18n";
	import type { InstanceDto } from "$lib/types/types";
	import { createMarketState } from "$lib/state/marketState.svelte";
	import type { ContentType } from "$lib/types/market";
	import MarketFilterPanel from "$lib/components/market/MarketFilterPanel.svelte";
	import MarketItem from "$lib/components/market/MarketItem.svelte";
	import MarketDetail from "$lib/components/market/MarketDetail.svelte";
	import MarketEmptyState from "$lib/components/market/MarketEmptyState.svelte";
	import MarketLayout from "$lib/components/market/MarketLayout.svelte";
	import InstalledToolbar from "$lib/components/market/InstalledToolbar.svelte";
	import InstalledItem from "$lib/components/market/InstalledItem.svelte";

	interface Props {
		instance: InstanceDto;
		contentType?: ContentType;
	}

	let { instance, contentType = "mods" }: Props = $props();

	function init() {
		return createMarketState(instance, contentType, () => instance.status);
	}
	const market = init();
	let installedView = $state<"list" | "cards">("list");
	let confirmingDelete = $state(false);
	let disposed = false;
	const localBusy = $derived(
		market.localOperationBusy || market.instanceBusy || confirmingDelete,
	);

	async function requestDelete(filenames: string[]) {
		if (localBusy || !filenames.length) return;
		confirmingDelete = true;
		try {
			const confirmed = await ask(
				t("market.manage.deleteConfirm", { count: filenames.length }) +
					"\n\n" +
					filenames.slice(0, 5).join("\n") +
					(filenames.length > 5 ? "\n…" : ""),
				{
					title: t("market.manage.deleteSelected"),
					kind: "warning",
					okLabel: t("market.detail.uninstall"),
					cancelLabel: t("market.detail.dependencies.cancel"),
				},
			);
			if (!disposed && confirmed)
				await market.manageLocal("delete", filenames);
		} catch (error) {
			if (!disposed) showErrorParsed(error);
		} finally {
			confirmingDelete = false;
		}
	}

	onDestroy(() => {
		disposed = true;
		try {
			market.destroy();
		} catch (e) {
			console.error("[Market] destroy error:", e);
		}
	});

	const emptyState = $derived.by(() => {
		if (
			market.filters.query.trim() ||
			(market.filters.source === "local"
				? market.filters.localSource !== "all" ||
					market.filters.localStatus !== "all"
				: market.filters.category !== null)
		) {
			return {
				title: t("market.empty.searchTitle"),
				subtitle: t("market.empty.searchSubtitle"),
			};
		}
		if (market.filters.source === "local") {
			return {
				title: t("market.empty.localTitle"),
				subtitle: t("market.empty.localSubtitle"),
			};
		}
		return {
			title: t("market.empty.marketTitle"),
			subtitle: t("market.empty.marketSubtitle"),
		};
	});
</script>

<div class="market-root">
	<MarketLayout
		listView={market.filters.source === "local" && installedView === "list"}
		items={market.items}
		itemCount={market.itemCount}
		getItem={market.getItem}
		onRangeNeeded={market.ensureRange}
		total={market.total}
		resultsRevision={market.resultsRevision}
		selectedId={market.selectedProject?.id ?? null}
		detailTitle={market.selectedProject?.title ?? ""}
		loading={market.loading}
		loadingMore={market.loadingMore}
		hasMore={market.hasMore}
		error={market.error}
		onClose={() => market.selectProject(null)}
		onRetry={market.retry}
		onLoadMore={market.loadMore}
	>
		{#snippet filterPanel()}
			<div inert={market.localOperationBusy || confirmingDelete}>
				<MarketFilterPanel
					filters={market.filters}
					{contentType}
					active={market.selectedProject === null &&
						!market.localOperationBusy}
					onSourceChange={market.setSource}
					onQueryChange={market.setQuery}
					onSearch={market.refresh}
					onSortChange={market.setSort}
					onCategoryChange={market.setCategory}
					onLocalSortChange={market.setLocalSort}
					onLocalSourceChange={market.setLocalSource}
					onClearFilters={market.clearFilters}
				/>
			</div>
			{#if market.filters.source === "local"}
				<InstalledToolbar
					{market}
					instanceId={instance.uuid}
					{contentType}
					bind:view={installedView}
					onDelete={requestDelete}
				/>
			{/if}
		{/snippet}

		{#snippet emptySnippet()}
			<MarketEmptyState
				title={emptyState.title}
				subtitle={emptyState.subtitle}
			/>
			<div class="empty-actions">
				{#if market.filters.query}
					<button type="button" onclick={() => market.setQuery("")}
						>{t("market.filter.clearSearch")}</button
					>
				{/if}
				{#if market.filters.source === "local" ? market.filters.localSource !== "all" || market.filters.localStatus !== "all" : market.filters.category !== null}
					<button type="button" onclick={market.clearFilters}
						>{t("market.browse.clearFilters")}</button
					>
				{/if}
			</div>
		{/snippet}

		{#snippet itemSnippet(project)}
			{#if market.filters.source === "local" && project.installed}
				{@const filename = project.installed.filename}
				<InstalledItem
					{project}
					icon={market.getLocalIcon(project)}
					checked={market.checkedFiles.has(filename)}
					compact={installedView === "list"}
					busy={localBusy}
					canToggle={contentType === "mods"}
					onCheck={() => market.toggleChecked(filename)}
					onOpen={() => market.selectProject(project.id)}
					onToggle={() =>
						market.manageLocal(
							project.disabled ? "enable" : "disable",
							[filename],
						)}
					onDelete={() => requestDelete([filename])}
				/>
			{:else}
				<MarketItem
					{project}
					selected={project.id === market.selectedId}
					onSelect={() => market.selectProject(project.id)}
					onInstall={market.filters.source !== "local"
						? () => market.selectProject(project.id)
						: undefined}
				/>
			{/if}
		{/snippet}

		{#snippet detailSnippet()}
			{#if market.selectedProject}
				{@const project = market.selectedProject}
				<MarketDetail
					{project}
					icon={market.getLocalIcon(project)}
					source={market.filters.source}
					{contentType}
					detail={market.detail}
					selectedVersion={market.selectedVersion}
					isVersionCompatible={market.isVersionCompatible}
					onVersionSelect={market.setSelectedVersion}
					onPrepareInstall={() => {
						const version = market.selectedVersion;
						if (!version) throw new Error("No version selected");
						return market.prepareInstall(project, version);
					}}
					onInstallQueue={(queue) =>
						market.confirmInstall(project, queue)}
					localActionsDisabled={localBusy}
					localActionError={market.localOperationReport?.failures[0]
						?.error}
					onUninstall={() =>
						project.installed &&
						requestDelete([project.installed.filename])}
					onToggleEnabled={() =>
						project.installed &&
						market.manageLocal(
							project.disabled ? "enable" : "disable",
							[project.installed.filename],
						)}
					onClose={() => market.selectProject(null)}
				/>
			{/if}
		{/snippet}
	</MarketLayout>
</div>

<style>
	.empty-actions {
		display: flex;
		gap: 10px;
		flex-wrap: wrap;
		justify-content: center;
	}
	.empty-actions button {
		padding: 8px 14px;
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		background: var(--surface-card);
		color: var(--text-primary);
		font: inherit;
		font-size: 0.8rem;
		cursor: pointer;
	}
	.empty-actions button:focus-visible {
		outline: 2px solid var(--accent);
		outline-offset: 2px;
	}
	.empty-actions button:hover {
		border-color: var(--accent);
	}
	.market-root {
		position: absolute;
		inset: 0;
		display: flex;
		flex-direction: column;
		overflow: hidden;
		background: var(--bg-main);
	}
</style>
