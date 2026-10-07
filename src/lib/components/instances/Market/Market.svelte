<script lang="ts">
	import { onDestroy } from "svelte";
	import { ask } from "@tauri-apps/plugin-dialog";
	import { showErrorParsed } from "$lib/state/state.svelte";
	import { t } from "$lib/i18n";
	import type { InstanceDto } from "$lib/types/types";
	import { createMarketState } from "$lib/state/marketState.svelte";
	import type { ContentType, MarketProject } from "$lib/types/market";
	import type {
		DependencyConflict,
		ResolvedDependency,
	} from "$lib/types/dependency";
	import type { ModDownloadInfo } from "$lib/api/cubicApi";
	import MarketFilterPanel from "$lib/components/market/MarketFilterPanel.svelte";
	import MarketDependenciesModal from "$lib/components/market/MarketDependenciesModal.svelte";
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

	interface InstallMenu {
		project: MarketProject;
		open: boolean;
		tree: ResolvedDependency[];
		conflicts: DependencyConflict[];
		installedProjectIds: Set<string>;
		resolving: boolean;
		downloading: boolean;
		error: string | null;
	}

	let installMenu = $state<InstallMenu | null>(null);
	let installMenuGeneration = 0;
	const installMenuBusy = $derived(
		installMenu !== null &&
			(installMenu.resolving || installMenu.downloading),
	);
	const localBusy = $derived(
		market.localOperationBusy ||
			market.quickInstallBusy ||
			market.instanceBusy ||
			confirmingDelete ||
			installMenuBusy,
	);

	function requestInstall(project: MarketProject) {
		// Mods preview their dependencies in the install menu; other content
		// types install straight away because their menu has nothing to show.
		if (contentType === "mods") return openInstallMenu(project);
		return market.installQuick(project).catch(() => {
			/* The card retains the error and offers retry. */
		});
	}

	async function openInstallMenu(project: MarketProject) {
		const generation = ++installMenuGeneration;
		installMenu = {
			project,
			open: true,
			tree: [],
			conflicts: [],
			installedProjectIds: new Set(),
			resolving: true,
			downloading: false,
			error: null,
		};
		try {
			const result = await market.resolveInstallPreview(project);
			if (generation !== installMenuGeneration || !installMenu) return;
			installMenu = {
				...installMenu,
				tree: result.tree,
				conflicts: result.conflicts,
				installedProjectIds: result.installedProjectIds,
				resolving: false,
			};
		} catch (error) {
			if (generation !== installMenuGeneration || !installMenu) return;
			installMenu = {
				...installMenu,
				resolving: false,
				error: error instanceof Error ? error.message : String(error),
			};
		}
	}

	async function confirmInstallMenu(queue: ModDownloadInfo[]) {
		const current = installMenu;
		if (!current || current.downloading) return;
		installMenu = { ...current, downloading: true, error: null };
		try {
			await market.confirmInstall(current.project, queue);
			if (installMenu?.project.id === current.project.id)
				installMenu = null;
		} catch (error) {
			if (installMenu?.project.id !== current.project.id) return;
			installMenu = {
				...installMenu,
				downloading: false,
				error: error instanceof Error ? error.message : String(error),
			};
		}
	}

	function closeInstallMenu() {
		installMenuGeneration++;
		installMenu = null;
	}

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
		installMenuGeneration++;
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
					market.filters.localStatus !== "all" ||
					(contentType === "mods" &&
						market.hasModpack &&
						market.filters.localOwnership !== "all")
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
				{#if market.filters.source === "local" && contentType === "mods" && market.hasModpack && market.filters.localOwnership !== "all"}
					<button
						type="button"
						onclick={() => market.setLocalOwnership("all")}
						>{t("modpack.showAll")}</button
					>
				{/if}
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
					onUpdate={market.canUpdateMod(project)
						? () => market.updateOwnMods([filename])
						: undefined}
					updating={market.modUpdateBusy &&
						market.modUpdateReport?.currentFile === filename}
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
					installing={market.quickInstallStatus(project) ===
						"preparing" ||
						market.quickInstallStatus(project) === "installing" ||
						(installMenu?.project.id === project.id &&
							installMenu.resolving)}
					installError={installMenu?.project.id === project.id
						? installMenu.error
						: market.quickInstallError(project)}
					installDisabled={localBusy}
					selected={project.id === market.selectedId}
					onSelect={() => market.selectProject(project.id)}
					onInstall={market.filters.source !== "local"
						? () => requestInstall(project)
						: undefined}
				/>
			{/if}
		{/snippet}

		{#snippet detailSnippet(closeDetail)}
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
					onRefreshVersions={() => market.selectProject(project.id)}
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
					onClose={closeDetail}
				/>
			{/if}
		{/snippet}
	</MarketLayout>

	{#if installMenu}
		<MarketDependenciesModal
			bind:open={installMenu.open}
			projectTitle={installMenu.project.title}
			replacing={false}
			tree={installMenu.tree}
			conflicts={installMenu.conflicts}
			installedProjectIds={installMenu.installedProjectIds}
			resolving={installMenu.resolving}
			downloading={installMenu.downloading}
			error={installMenu.error}
			onConfirm={confirmInstallMenu}
			onCancel={closeInstallMenu}
			onclose={closeInstallMenu}
		/>
	{/if}
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
