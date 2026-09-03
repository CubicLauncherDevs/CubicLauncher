<script lang="ts">
	import { onMount } from "svelte";

	import {
		getAvailableVersions,
		addToQueue,
		getFabricVersions,
		getFabricLoaderVersions,
		downloadFabric,
		getForgeVersions,
		downloadForge,
		getNeoForgeVersions,
		downloadNeoForge,
		getQuiltVersions,
		getQuiltLoaderVersions,
		downloadQuilt,
		refreshAvailableVersions,
		refreshForgeVersions,
		refreshNeoForgeVersions,
	} from "$lib/api/cubicApi";
	import {
		versionsState,
		loadInstalledVersions,
		invalidateInstalledVersions,
	} from "$lib/state/versionsState.svelte";
	import type {
		MinecraftVersion,
		ForgeGameVersion,
		NeoForgeGameVersion,
	} from "$lib/types/types";
	import { onAppEvent } from "$lib/api/launcherService";
	import { SvelteSet } from "svelte/reactivity";
	import { isVersionDownloading } from "$lib/state/downloadState.svelte";
	import { launcherStore } from "$lib/state/state.svelte";
	import { t } from "$lib/i18n";
	import Select from "../Select.svelte";
	import ModalBase from "../ModalBase.svelte";
	import VirtualList from "../VirtualList.svelte";
	import VersionDownloaderTabs from "./VersionDownloaderTabs.svelte";
	import Icon from "$lib/icons/Icon.svelte";
	import Tooltip from "$lib/components/ui/Tooltip.svelte";

	let { open = $bindable(false) }: { open: boolean } = $props();

	let tooltipOpen = $state(false);
	let tooltipX = $state(0);
	let tooltipY = $state(0);
	let tooltipText = $state("");

	function showStableTooltip(e: MouseEvent | FocusEvent, text: string) {
		const target = e.currentTarget as HTMLElement;
		const rect = target.getBoundingClientRect();
		tooltipX = rect.right;
		tooltipY = rect.top;
		tooltipText = text;
		tooltipOpen = true;
	}

	function hideStableTooltip() {
		tooltipOpen = false;
	}

	const LOADERS = [
		{
			value: "vanilla",
			label: "Vanilla",
			iconName: "brand:vanilla",
		},
		{
			value: "fabric",
			label: "Fabric",
			iconName: "brand:fabric",
		},
		{
			value: "forge",
			label: "Forge",
			iconName: "brand:forge",
		},
		{
			value: "neoforge",
			label: "NeoForge",
			iconName: "brand:neoforged",
		},
		{
			value: "quilt",
			label: "Quilt",
			iconName: "brand:quilt",
		},
	];

	let loaderTab = $state("vanilla");
	let refreshing = $state(false);

	// --- Vanilla tab state ---
	let vanillaSearch = $state("");
	let loadingMojang = $state(false);
	let loadingVanillaInstalled = $derived(versionsState.loading);

	// --- Loader tabs state (fabric, forge, neoforge, quilt) ---
	let mcVersions = $state<string[]>([]);
	let selectedMcVersion = $state("");
	let loaderItems = $state<LoaderDisplayItem[]>([]);
	let loadingMinecraft = $state(false);
	let loadingLoader = $state(false);
	let mcLoadId = $state(0);
	let loaderLoadId = $state(0);

	// --- Cached full lists for forge/neoforge ---
	let forgeCache = $state<ForgeGameVersion[]>([]);
	let neoForgeCache = $state<NeoForgeGameVersion[]>([]);

	let loaderSearch = $state("");

	// --- Vanilla manifest cache for "Show all" ---
	let vanillaAllCache = $state<MinecraftVersion[] | null>(null);

	interface LoaderDisplayItem {
		version_id: string;
		display_version: string;
		game_version: string;
		stable: boolean;
	}

	// --- Helpers ---

	function compareVersions(a: string, b: string): number {
		const aParts = a.split(".").map((n) => parseInt(n, 10) || 0);
		const bParts = b.split(".").map((n) => parseInt(n, 10) || 0);
		for (let i = 0; i < Math.max(aParts.length, bParts.length); i++) {
			const av = aParts[i] ?? 0;
			const bv = bParts[i] ?? 0;
			if (av !== bv) return bv - av;
		}
		return b.localeCompare(a, undefined, { numeric: true });
	}

	// --- Tab switching ---

	async function switchTab(tab: string) {
		loaderTab = tab;
		if (tab === "vanilla") return;

		const currentLoadId = ++mcLoadId;
		++loaderLoadId;
		loadingMinecraft = true;
		loadingLoader = true;
		mcVersions = [];
		loaderItems = [];
		selectedMcVersion = "";
		loaderSearch = "";

		try {
			await loadInstalledVersions();
			if (currentLoadId !== mcLoadId) return;

			let mcList: string[] = [];

			if (tab === "forge") {
				if (forgeCache.length === 0)
					forgeCache = await getForgeVersions();
				const seen = new SvelteSet<string>();
				for (const v of forgeCache) {
					if (v.game_version && !seen.has(v.game_version)) {
						seen.add(v.game_version);
						mcList.push(v.game_version);
					}
				}
			} else if (tab === "neoforge") {
				if (neoForgeCache.length === 0)
					neoForgeCache = await getNeoForgeVersions();
				const seen = new SvelteSet<string>();
				for (const v of neoForgeCache) {
					if (v.game_version && !seen.has(v.game_version)) {
						seen.add(v.game_version);
						mcList.push(v.game_version);
					}
				}
			} else {
				const list =
					tab === "fabric"
						? await getFabricVersions()
						: await getQuiltVersions();
				const seen = new SvelteSet<string>();
				for (const v of list) {
					if (v.version && v.stable && !seen.has(v.version)) {
						seen.add(v.version);
						mcList.push(v.version);
					}
				}
			}

			if (currentLoadId !== mcLoadId) return;
			mcList.sort(compareVersions);
			mcVersions = mcList;
			if (mcList.length > 0) {
				selectedMcVersion = mcList[0];
				await loadLoaderVersions(selectedMcVersion, tab);
			} else {
				loadingLoader = false;
			}
		} catch {
			if (currentLoadId !== mcLoadId) return;
		} finally {
			if (currentLoadId === mcLoadId) {
				loadingMinecraft = false;
			}
		}
	}

	async function loadLoaderVersions(mcVersion: string, loader: string) {
		const currentLoadId = ++loaderLoadId;
		loadingLoader = true;
		loaderItems = [];

		try {
			let items: LoaderDisplayItem[] = [];

			const showUnstable = launcherStore.settings.show_unstable_loaders;

			if (loader === "fabric") {
				const list = await getFabricLoaderVersions(mcVersion);
				for (const lv of list.filter((v) => showUnstable || v.stable)) {
					const vid = `fabric-loader-${lv.version}-${mcVersion}`;
					items.push({
						version_id: vid,
						display_version: lv.version,
						game_version: mcVersion,
						stable: lv.stable,
					});
				}
			} else if (loader === "quilt") {
				const list = await getQuiltLoaderVersions(mcVersion);
				for (const lv of list.filter((v) => showUnstable || v.stable)) {
					const vid = `quilt-loader-${lv.version}-${mcVersion}`;
					items.push({
						version_id: vid,
						display_version: lv.version,
						game_version: mcVersion,
						stable: lv.stable,
					});
				}
			} else if (loader === "forge") {
				for (const v of forgeCache) {
					if (
						v.game_version !== mcVersion ||
						!(showUnstable || v.stable)
					)
						continue;
					items.push({
						version_id: v.version_id,
						display_version: v.forge_version,
						game_version: mcVersion,
						stable: v.stable,
					});
				}
			} else if (loader === "neoforge") {
				for (const v of neoForgeCache) {
					if (
						v.game_version !== mcVersion ||
						!(showUnstable || v.stable)
					)
						continue;
					items.push({
						version_id: v.version_id,
						display_version: v.neoforge_version,
						game_version: mcVersion,
						stable: v.stable,
					});
				}
			}

			if (currentLoadId !== loaderLoadId) return;

			items.sort(
				(a, b) =>
					Number(b.stable) - Number(a.stable) ||
					compareVersions(a.display_version, b.display_version),
			);
			loaderItems = items;
		} catch {
			if (currentLoadId !== loaderLoadId) return;
		} finally {
			if (currentLoadId === loaderLoadId) loadingLoader = false;
		}
	}

	// --- Vanilla tab ---

	async function loadVanillaInstalled() {
		await loadInstalledVersions();
	}

	async function loadAllVanillaVersions() {
		if (vanillaAllCache) return;
		loadingMojang = true;
		try {
			vanillaAllCache = await getAvailableVersions();
		} catch {
			// ignore
		} finally {
			loadingMojang = false;
		}
	}

	const normalizedVanillaSearch = $derived(
		vanillaSearch.trim().toLowerCase(),
	);

	const vanillaDisplayList = $derived.by(() => {
		if (!vanillaAllCache) return [];
		return vanillaAllCache.filter((v) => {
			if (
				normalizedVanillaSearch &&
				!v.id.toLowerCase().includes(normalizedVanillaSearch)
			)
				return false;
			if (!launcherStore.settings.show_snapshots && v.type === "snapshot")
				return false;
			if (
				!launcherStore.settings.show_alpha &&
				(v.type === "old_alpha" || v.type === "old_beta")
			)
				return false;
			return true;
		});
	});

	// --- Downloads ---

	async function handleDownloadVanilla(versionId: string) {
		await addToQueue(versionId);
		invalidateInstalledVersions();
	}

	async function handleDownloadLoader(item: LoaderDisplayItem) {
		const mc = item.game_version;
		const lv = item.display_version;
		if (loaderTab === "fabric") {
			await downloadFabric(mc, lv);
		} else if (loaderTab === "quilt") {
			await downloadQuilt(mc, lv);
		} else if (loaderTab === "forge") {
			await downloadForge(mc, lv);
		} else if (loaderTab === "neoforge") {
			await downloadNeoForge(mc, lv);
		}
		invalidateInstalledVersions();
	}

	const mcVersionOptions = $derived(
		mcVersions.map((v) => ({ value: v, label: v })),
	);

	const mcPlaceholder = $derived(
		!loadingMinecraft && mcVersions.length === 0
			? t("versionDownloader.notFound")
			: t("createInstance.selectMcVersion"),
	);

	const normalizedLoaderSearch = $derived(loaderSearch.trim().toLowerCase());

	const filteredLoaderItems = $derived.by(() => {
		if (!normalizedLoaderSearch) return loaderItems;
		return loaderItems.filter(
			(item) =>
				item.display_version
					.toLowerCase()
					.includes(normalizedLoaderSearch) ||
				item.game_version
					.toLowerCase()
					.includes(normalizedLoaderSearch),
		);
	});

	// --- Refresh ---

	async function refreshCurrentSource() {
		if (loaderTab === "vanilla") {
			refreshing = true;
			try {
				vanillaAllCache = await refreshAvailableVersions();
				await loadVanillaInstalled();
			} finally {
				refreshing = false;
			}
		} else if (loaderTab === "fabric" || loaderTab === "quilt") {
			refreshing = true;
			try {
				await loadLoaderVersions(selectedMcVersion, loaderTab);
			} finally {
				refreshing = false;
			}
		} else if (loaderTab === "forge") {
			refreshing = true;
			try {
				forgeCache = await refreshForgeVersions();
				await switchTab("forge");
			} finally {
				refreshing = false;
			}
		} else if (loaderTab === "neoforge") {
			refreshing = true;
			try {
				neoForgeCache = await refreshNeoForgeVersions();
				await switchTab("neoforge");
			} finally {
				refreshing = false;
			}
		}
	}

	// --- Lifecycle ---

	onMount(() => {
		loadVanillaInstalled();
		loadAllVanillaVersions();

		const unsubFinish = onAppEvent("DFinish", async () => {
			await loadVanillaInstalled();
			if (loaderTab !== "vanilla") {
				await loadLoaderVersions(selectedMcVersion, loaderTab);
			}
		});

		return () => {
			unsubFinish();
		};
	});
</script>

<ModalBase bind:open title={t("versionDownloader.title")} width="700px">
	<div class="version-downloader-body">
		<div class="vd-header">
			<button
				type="button"
				class="vd-refresh-btn"
				onclick={refreshCurrentSource}
				disabled={refreshing}
				title={t("versionDownloader.refreshBtn")}
			>
				<svg
					width="16"
					height="16"
					viewBox="0 0 24 24"
					fill="none"
					stroke="currentColor"
					stroke-width="2"
					stroke-linecap="round"
					stroke-linejoin="round"
					class:spin={refreshing}
				>
					<polyline points="23 4 23 10 17 10"></polyline>
					<path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10"></path>
				</svg>
			</button>
		</div>

		<div class="vd-layout">
			<div class="vd-loader-sidebar">
				<VersionDownloaderTabs
					bind:loaderTab
					{LOADERS}
					onswitch={switchTab}
				/>
			</div>

			<div class="vd-content">
				{#if loaderTab === "vanilla"}
					<div class="vd-tab-content">
						<div class="vd-search">
							<input
								type="text"
								class="text-input"
								placeholder={t(
									"versionDownloader.searchPlaceholder",
								)}
								bind:value={vanillaSearch}
							/>
						</div>

						{#if loadingMojang || loadingVanillaInstalled}
							<div class="qm-empty-state">
								{t("versionDownloader.loading")}
							</div>
						{:else if vanillaDisplayList.length === 0}
							<div class="qm-empty-state">
								{t("versionDownloader.notFound")}
							</div>
						{:else}
							<VirtualList
								items={vanillaDisplayList}
								itemHeight={64}
								keyFn={(v) => v.id}
								class="vd-virtual-list"
							>
								{#snippet children(vitem)}
									{@const vid = vitem.id}
									{@const isVanInstalled =
										versionsState.mcVersions?.vanilla.has(
											vid,
										) ?? false}
									{@const isVanDownloading =
										isVersionDownloading(vid)}
									<div class="version-card">
										<div class="version-card-info">
											<div class="version-card-name">
												{vid}
											</div>
											<div class="version-card-type">
												{vitem.type ?? "release"} • {new Date(
													vitem.releaseTime ?? "",
												).toLocaleDateString()}
											</div>
										</div>
										{#if isVanInstalled}
											<div class="inst-icon">✓</div>
										{:else if isVanDownloading}
											<button
												type="button"
												class="download-btn"
												disabled
											>
												<span class="dl-spinner"></span>
												{t(
													"versionDownloader.downloading",
												)}
											</button>
										{:else}
											<button
												type="button"
												class="download-btn"
												onclick={() =>
													handleDownloadVanilla(vid)}
											>
												{t(
													"versionDownloader.downloadBtn",
												)}
											</button>
										{/if}
									</div>
								{/snippet}
							</VirtualList>
						{/if}
					</div>
				{:else}
					<div class="vd-tab-content">
						<div class="vd-controls">
							<div class="vd-control linked-selects">
								<Select
									bind:value={selectedMcVersion}
									options={mcVersionOptions}
									placeholder={mcPlaceholder}
									loading={loadingMinecraft}
									loadingPlaceholder={t(
										"createInstance.loading",
									)}
									disabled={loadingMinecraft ||
										mcVersionOptions.length === 0}
									onchange={(value) =>
										loadLoaderVersions(value, loaderTab)}
								/>
							</div>

							<div class="vd-control vd-search">
								<input
									type="text"
									class="text-input"
									placeholder={t(
										"versionDownloader.loaderSearchPlaceholder",
									)}
									bind:value={loaderSearch}
								/>
							</div>
						</div>

						{#if loaderTab === "forge" || loaderTab === "neoforge"}
							<div class="vd-hint">
								{t(
									loaderTab === "neoforge"
										? "versionDownloader.neoForgeJavaHint"
										: "versionDownloader.forgeJavaHint",
								)}
							</div>
						{/if}

						{#if loadingLoader}
							<div class="qm-empty-state">
								{t("versionDownloader.loading")}
							</div>
						{:else if filteredLoaderItems.length === 0}
							<div class="qm-empty-state">
								{t("versionDownloader.notFound")}
							</div>
						{:else}
							<VirtualList
								items={filteredLoaderItems}
								itemHeight={64}
								keyFn={(item) => item.version_id}
								class="vd-virtual-list"
							>
								{#snippet children(item)}
									{@const isInstalled =
										versionsState.mcVersions?.[
											loaderTab as keyof typeof versionsState.mcVersions
										].has(item.version_id) ?? false}
									{@const isDownloading =
										isVersionDownloading(item.version_id)}
									<div class="version-card">
										<div class="version-card-info">
											<div class="version-card-name">
												{#if item.stable}
													{@const tooltipText = t(
														"versionDownloader.stableTooltip",
													)}
													<span
														class="stable-icon"
														role="img"
														aria-label={tooltipText}
														onmouseenter={(e) =>
															showStableTooltip(
																e,
																tooltipText,
															)}
														onmouseleave={hideStableTooltip}
														onfocus={(e) =>
															showStableTooltip(
																e,
																tooltipText,
															)}
														onblur={hideStableTooltip}
													>
														<Icon
															name="ui:check-circle"
															size={14}
														/>
													</span>
												{/if}
												{item.display_version}
											</div>
											<div class="version-card-type">
												{loaderTab === "fabric"
													? "Fabric"
													: loaderTab === "quilt"
														? "Quilt"
														: loaderTab === "forge"
															? "Forge"
															: "NeoForge"} • MC {item.game_version}
											</div>
										</div>
										{#if isInstalled}
											<div class="inst-icon">✓</div>
										{:else if isDownloading}
											<button
												type="button"
												class="download-btn"
												disabled
											>
												<span class="dl-spinner"></span>
												{t(
													"versionDownloader.downloading",
												)}
											</button>
										{:else}
											<button
												type="button"
												class="download-btn"
												onclick={() =>
													handleDownloadLoader(item)}
											>
												{t(
													"versionDownloader.downloadBtn",
												)}
											</button>
										{/if}
									</div>
								{/snippet}
							</VirtualList>
						{/if}
					</div>
				{/if}
			</div>
		</div>
	</div>
</ModalBase>

<Tooltip bind:open={tooltipOpen} x={tooltipX} y={tooltipY} placement="right">
	{tooltipText}
</Tooltip>

<style>
	.version-downloader-body {
		display: flex;
		flex-direction: column;
		gap: 12px;
		height: 100%;
		min-height: 0;
	}

	.vd-header {
		display: flex;
		justify-content: flex-end;
	}

	.vd-refresh-btn {
		background: none;
		border: none;
		color: var(--text-muted);
		cursor: pointer;
		padding: 4px;
		display: flex;
		align-items: center;
		border-radius: var(--border-radius-sm);
		transition: color 0.2s;
	}

	.vd-refresh-btn:hover:not(:disabled) {
		color: var(--text-primary);
	}

	.vd-refresh-btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.vd-refresh-btn svg {
		display: block;
	}

	.vd-refresh-btn svg.spin {
		animation: vd-spin 1s linear infinite;
	}

	.vd-layout {
		display: flex;
		flex: 1;
		gap: 16px;
		min-height: 0;
	}

	.vd-loader-sidebar {
		width: 110px;
		flex-shrink: 0;
		display: flex;
		flex-direction: column;
	}

	@media (max-width: 500px) {
		.vd-loader-sidebar {
			width: 64px;
		}
	}

	.vd-content {
		flex: 1;
		min-height: 0;
		overflow: hidden;
		display: flex;
		flex-direction: column;
	}

	.vd-tab-content {
		display: flex;
		flex-direction: column;
		gap: 12px;
		flex: 1;
		min-height: 0;
	}

	.vd-controls {
		display: flex;
		gap: 12px;
		flex-wrap: wrap;
		flex-shrink: 0;
	}

	.vd-control {
		flex: 1;
		min-width: 160px;
		display: flex;
		flex-direction: column;
	}

	.vd-search {
		width: 100%;
		flex-shrink: 0;
	}

	.vd-search .text-input {
		width: 100%;
	}

	.vd-hint {
		font-size: 0.75rem;
		color: var(--text-muted);
		padding: 0 4px;
		flex-shrink: 0;
	}

	:global(.vd-virtual-list) {
		flex: 1;
		min-height: 0;
	}

	:global(.vd-virtual-list .virtual-list-item-wrapper) {
		padding: 3px 0;
		box-sizing: border-box;
	}

	@keyframes vd-spin {
		to {
			transform: rotate(360deg);
		}
	}

	.qm-empty-state {
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 40px 20px;
		color: var(--text-muted);
		font-size: 0.85rem;
		text-align: center;
	}

	.version-card {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 10px 12px;
		background: var(--bg-card);
		border: 1px solid var(--border-color);
		border-radius: var(--border-radius);
		gap: 8px;
		transition:
			background-color 0.15s ease,
			border-color 0.15s ease;
	}

	.version-card:hover {
		background: var(--surface-hover);
		border-color: rgba(var(--surface-rgb), 0.2);
	}

	.version-card-info {
		display: flex;
		flex-direction: column;
		gap: 2px;
		min-width: 0;
		flex: 1;
	}

	.version-card-name {
		font-weight: 600;
		font-size: 0.85rem;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.version-card-type {
		font-size: 0.65rem;
		color: var(--text-muted);
		text-transform: uppercase;
		letter-spacing: 0.3px;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.stable-icon {
		color: var(--accent);
		display: inline-flex;
		align-items: center;
		vertical-align: middle;
		margin-right: 4px;
		flex-shrink: 0;
		cursor: help;
	}

	.inst-icon {
		color: var(--color-success);
		padding: 4px 8px;
		font-size: 1rem;
		font-weight: 700;
		flex-shrink: 0;
	}

	.download-btn {
		background: var(--accent);
		color: var(--accent-text);
		border: 1px solid transparent;
		padding: 6px 12px;
		border-radius: var(--border-radius-sm);
		font-size: 0.75rem;
		font-weight: 700;
		cursor: pointer;
		transition:
			background-color 0.2s,
			opacity 0.2s;
		display: inline-flex;
		align-items: center;
		gap: 6px;
		flex-shrink: 0;
	}

	.download-btn:hover:not(:disabled) {
		opacity: 0.9;
	}

	.download-btn:disabled {
		opacity: 0.6;
		cursor: not-allowed;
		background: var(--bg-input);
		color: var(--text-muted);
		border-color: var(--border-color);
	}

	.dl-spinner {
		width: 12px;
		height: 12px;
		border: 1.5px solid var(--border);
		border-top-color: var(--text-muted);
		border-radius: 50%;
		animation: dl-spin 0.7s linear infinite;
		will-change: transform;
		flex-shrink: 0;
	}

	@keyframes dl-spin {
		to {
			transform: rotate(360deg);
		}
	}

	.linked-selects {
		width: 100%;
	}

	.linked-selects > :global(.custom-select-container) {
		width: 100%;
	}
</style>
