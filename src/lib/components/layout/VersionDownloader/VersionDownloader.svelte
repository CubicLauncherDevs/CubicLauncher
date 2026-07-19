<script lang="ts">
	import { onMount } from "svelte";
	import { SvelteSet } from "svelte/reactivity";

	import {
		getInstalledVersions,
		getInstalledMcVersions,
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
	import type {
		MinecraftVersion,
		ForgeGameVersion,
		NeoForgeGameVersion,
	} from "$lib/types/types";
	import { onAppEvent } from "$lib/api/launcherService";
	import { isVersionDownloading } from "$lib/state/downloadState.svelte";
	import { launcherStore } from "$lib/state/state.svelte";
	import { t } from "$lib/i18n";
	import Select from "../Select.svelte";
	import ModalBase from "../ModalBase.svelte";
	import VersionDownloaderTabs from "./VersionDownloaderTabs.svelte";

	let { open = $bindable(false) }: { open: boolean } = $props();

	const LOADERS = [
		{
			value: "vanilla",
			label: "Vanilla",
			icon: "/images/instances/vanilla.png",
		},
		{
			value: "fabric",
			label: "Fabric",
			icon: "/images/instances/fabric.png",
		},
		{ value: "forge", label: "Forge", icon: "/images/instances/forge.png" },
		{
			value: "neoforge",
			label: "NeoForge",
			icon: "/images/instances/neoforged.png",
		},
		{ value: "quilt", label: "Quilt", icon: "/images/instances/quilt.png" },
	];

	let loaderTab = $state("vanilla");
	let refreshing = $state(false);

	// --- Installed versions (shared) ---
	let installed = $state({
		vanilla: new Set<string>(),
		fabric: new Set<string>(),
		forge: new Set<string>(),
		neoforge: new Set<string>(),
		quilt: new Set<string>(),
	});

	// --- Vanilla tab state ---
	let vanillaSearch = $state("");
	let vanillaShowAll = $state(false);
	let loadingMojang = $state(false);
	let loadingVanillaInstalled = $state(true);

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

	let fabricQuiltShowAll = $state(false);

	// --- Vanilla manifest cache for "Show all" ---
	let vanillaAllCache = $state<MinecraftVersion[] | null>(null);

	interface LoaderDisplayItem {
		version_id: string;
		display_version: string;
		game_version: string;
		stable: boolean;
		isInstalled: boolean;
		isDownloading: boolean;
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
		fabricQuiltShowAll = false;

		try {
			const raw = await getInstalledVersions();
			if (currentLoadId !== mcLoadId) return;
			installed = getInstalledMcVersions(raw);

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
				fabricQuiltShowAll = true;
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

			if (loader === "fabric") {
				const list = await getFabricLoaderVersions(mcVersion);
				for (const lv of list) {
					const vid = `fabric-loader-${lv}-${mcVersion}`;
					items.push({
						version_id: vid,
						display_version: lv,
						game_version: mcVersion,
						stable: false,
						isInstalled: installed.fabric.has(vid),
						isDownloading: isVersionDownloading(vid),
					});
				}
			} else if (loader === "quilt") {
				const list = await getQuiltLoaderVersions(mcVersion);
				for (const lv of list) {
					const vid = `quilt-loader-${lv}-${mcVersion}`;
					items.push({
						version_id: vid,
						display_version: lv,
						game_version: mcVersion,
						stable: false,
						isInstalled: installed.quilt.has(vid),
						isDownloading: isVersionDownloading(vid),
					});
				}
			} else if (loader === "forge") {
				for (const v of forgeCache) {
					if (v.game_version !== mcVersion) continue;
					const vid = v.version_id;
					items.push({
						version_id: vid,
						display_version: v.forge_version,
						game_version: mcVersion,
						stable: true,
						isInstalled: installed.forge.has(vid),
						isDownloading: isVersionDownloading(vid),
					});
				}
			} else if (loader === "neoforge") {
				for (const v of neoForgeCache) {
					if (v.game_version !== mcVersion) continue;
					const vid = v.version_id;
					items.push({
						version_id: vid,
						display_version: v.neoforge_version,
						game_version: mcVersion,
						stable: true,
						isInstalled: installed.neoforge.has(vid),
						isDownloading: isVersionDownloading(vid),
					});
				}
			}

			if (currentLoadId !== loaderLoadId) return;

			items.sort((a, b) => {
				return compareVersions(a.display_version, b.display_version);
			});
			loaderItems = items;
		} catch {
			if (currentLoadId !== loaderLoadId) return;
		} finally {
			if (currentLoadId === loaderLoadId) loadingLoader = false;
		}
	}

	// --- Vanilla tab ---

	async function loadVanillaInstalled() {
		loadingVanillaInstalled = true;
		try {
			const raw = await getInstalledVersions();
			installed = getInstalledMcVersions(raw);
		} finally {
			loadingVanillaInstalled = false;
		}
	}

	async function showAllVanilla() {
		if (vanillaAllCache) {
			vanillaShowAll = true;
			return;
		}
		loadingMojang = true;
		try {
			vanillaAllCache = await getAvailableVersions();
			vanillaShowAll = true;
		} catch {
			// ignore
		} finally {
			loadingMojang = false;
		}
	}

	const vanillaDisplayList = $derived.by(() => {
		if (vanillaShowAll && vanillaAllCache) {
			return vanillaAllCache.filter((v) => {
				if (
					vanillaSearch &&
					!v.id.toLowerCase().includes(vanillaSearch.toLowerCase())
				)
					return false;
				if (
					!launcherStore.settings.show_snapshots &&
					v.type === "snapshot"
				)
					return false;
				if (
					!launcherStore.settings.show_alpha &&
					(v.type === "old_alpha" || v.type === "old_beta")
				)
					return false;
				return true;
			});
		}
		const list = Array.from(installed.vanilla);
		const filtered = vanillaSearch
			? list.filter((v) =>
					v.toLowerCase().includes(vanillaSearch.toLowerCase()),
				)
			: list;
		return filtered.sort(compareVersions).map(
			(id) =>
				({
					id,
					type: "release",
					url: "",
					time: "",
					releaseTime: "",
				}) as MinecraftVersion,
		);
	});

	// --- Downloads ---

	async function handleDownloadVanilla(versionId: string) {
		await addToQueue(versionId);
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
	}

	const mcVersionOptions = $derived(
		mcVersions.map((v) => ({ value: v, label: v })),
	);

	const mcPlaceholder = $derived(
		!loadingMinecraft && mcVersions.length === 0
			? t("versionDownloader.notFound")
			: t("createInstance.selectMcVersion"),
	);

	// --- Refresh ---

	async function refreshCurrentSource() {
		if (loaderTab === "vanilla") {
			refreshing = true;
			try {
				if (vanillaShowAll) {
					vanillaAllCache = await refreshAvailableVersions();
				}
				await loadVanillaInstalled();
			} finally {
				refreshing = false;
			}
		} else if (loaderTab === "fabric" && fabricQuiltShowAll) {
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
		} else if (loaderTab === "quilt" && fabricQuiltShowAll) {
			refreshing = true;
			try {
				await loadLoaderVersions(selectedMcVersion, loaderTab);
			} finally {
				refreshing = false;
			}
		}
	}

	// --- Lifecycle ---

	onMount(() => {
		loadVanillaInstalled();

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
		<div style="display: flex; justify-content: flex-end;">
			<button
				type="button"
				onclick={refreshCurrentSource}
				disabled={refreshing}
				title={t("versionDownloader.refreshBtn")}
				style="background: none; border: none; color: var(--text-muted); cursor: pointer; padding: 4px; display: flex; align-items: center; border-radius: 4px; transition: color 0.2s;"
				onmouseenter={(e) =>
					(e.currentTarget.style.color = "var(--text-primary)")}
				onmouseleave={(e) =>
					(e.currentTarget.style.color = "var(--text-muted)")}
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
					style={refreshing
						? "animation: spin 1s linear infinite; will-change: transform;"
						: ""}
				>
					<polyline points="23 4 23 10 17 10"></polyline>
					<path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10"></path>
				</svg>
			</button>
		</div>

		<VersionDownloaderTabs bind:loaderTab {LOADERS} onswitch={switchTab} />

		<div class="vd-scrollable">
			{#if loaderTab === "vanilla"}
				<div style="display: flex; flex-direction: column; gap: 12px;">
					<input
						type="text"
						placeholder={t("versionDownloader.searchPlaceholder")}
						bind:value={vanillaSearch}
						style="width: 100%; background: var(--bg-input); border: 1px solid var(--border-color); color: var(--text-primary); padding: 8px 12px; border-radius: 8px; font-size: 0.85rem; box-sizing: border-box;"
					/>

					{#if !vanillaShowAll}
						<button
							type="button"
							class="browse-all-btn"
							onclick={showAllVanilla}
							disabled={loadingMojang}
						>
							{loadingMojang
								? t("versionDownloader.loading")
								: t("versionDownloader.showAll")}
						</button>
					{/if}

					{#if loadingVanillaInstalled}
						<div class="qm-empty-state">
							{t("versionDownloader.loading")}
						</div>
					{:else if vanillaDisplayList.length === 0}
						<div class="qm-empty-state">
							{t("versionDownloader.notFound")}
						</div>
					{:else}
						<div
							style="display: flex; flex-direction: column; gap: 6px;"
						>
							{#each vanillaDisplayList as vitem (vitem.id)}
								{@const vid = vitem.id}
								{@const isVanInstalled =
									installed.vanilla.has(vid)}
								{@const isVanDownloading =
									isVersionDownloading(vid)}
								<div class="version-card">
									<div class="version-card-info">
										<div class="version-card-name">
											{vid}
										</div>
										{#if !vanillaShowAll}
											<div
												class="version-card-badge installed-badge"
											>
												{t(
													"versionDownloader.installedTag",
												)}
											</div>
										{:else}
											<div class="version-card-type">
												{(vitem as MinecraftVersion)
													.type ?? "release"} • {new Date(
													(vitem as MinecraftVersion)
														.releaseTime ?? "",
												).toLocaleDateString()}
											</div>
										{/if}
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
											{t("versionDownloader.downloading")}
										</button>
									{:else}
										<button
											type="button"
											class="download-btn"
											onclick={() =>
												handleDownloadVanilla(vid)}
										>
											{t("versionDownloader.downloadBtn")}
										</button>
									{/if}
								</div>
							{/each}
						</div>
					{/if}
				</div>
			{:else}
				<div style="display: flex; flex-direction: column; gap: 16px;">
					<div class="linked-selects">
						<Select
							bind:value={selectedMcVersion}
							options={mcVersionOptions}
							placeholder={mcPlaceholder}
							loading={loadingMinecraft}
							loadingPlaceholder={t("createInstance.loading")}
							disabled={loadingMinecraft ||
								mcVersionOptions.length === 0}
							onchange={(value) =>
								loadLoaderVersions(value, loaderTab)}
						/>
					</div>

					{#if loaderTab === "forge" || loaderTab === "neoforge"}
						<div
							style="font-size: 0.75rem; color: var(--text-muted); padding: 0 4px;"
						>
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
					{:else if loaderItems.length === 0}
						<div class="qm-empty-state">
							{t("versionDownloader.notFound")}
						</div>
					{:else}
						<div
							style="display: flex; flex-direction: column; gap: 6px;"
						>
							{#each loaderItems as item (item.version_id)}
								<div class="version-card">
									<div class="version-card-info">
										<div class="version-card-name">
											{item.display_version}
										</div>
										<div class="version-card-type">
											{loaderTab === "fabric" ||
											loaderTab === "quilt"
												? `${item.stable ? "STABLE" : "UNSTABLE"}`
												: `${loaderTab === "forge" ? "Forge" : "NeoForge"} • MC ${item.game_version}`}
										</div>
									</div>
									{#if item.isInstalled}
										<div class="inst-icon">✓</div>
									{:else if item.isDownloading}
										<button
											type="button"
											class="download-btn"
											disabled
										>
											<span class="dl-spinner"></span>
											{t("versionDownloader.downloading")}
										</button>
									{:else}
										<button
											type="button"
											class="download-btn"
											onclick={() =>
												handleDownloadLoader(item)}
										>
											{t("versionDownloader.downloadBtn")}
										</button>
									{/if}
								</div>
							{/each}
						</div>
					{/if}
				</div>
			{/if}
		</div>
	</div>
</ModalBase>

<style>
	.version-downloader-body {
		display: flex;
		flex-direction: column;
		gap: 16px;
		height: 100%;
		min-height: 0;
	}

	.vd-scrollable {
		flex: 1;
		overflow-y: auto;
		min-height: 0;
		max-height: 440px;
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
		border-radius: 8px;
		gap: 8px;
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

	.version-card-badge {
		font-size: 0.5rem;
		padding: 1px 5px;
		border-radius: 3px;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 0.3px;
	}

	.installed-badge {
		background: rgba(var(--color-success-rgb), 0.1);
		color: var(--color-success);
		border: 1px solid rgba(var(--color-success-rgb), 0.2);
		align-self: flex-start;
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
		border: none;
		padding: 4px 10px;
		border-radius: var(--border-radius-sm);
		font-size: 0.7rem;
		font-weight: 700;
		cursor: pointer;
		transition: all 0.2s;
		display: flex;
		align-items: center;
		gap: 5px;
		flex-shrink: 0;
	}

	.download-btn:hover {
		opacity: 0.9;
	}

	.download-btn:disabled {
		opacity: 0.6;
		cursor: not-allowed;
		background: var(--bg-input);
		color: var(--text-muted);
		border: 1px solid var(--border-color);
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

	.browse-all-btn {
		background: none;
		border: 1px solid var(--border-color);
		color: var(--accent);
		padding: 8px 16px;
		border-radius: var(--border-radius-sm);
		font-size: 0.8rem;
		font-weight: 600;
		cursor: pointer;
		transition: all 0.15s;
		text-align: center;
		width: 100%;
	}

	.browse-all-btn:hover {
		background: rgba(var(--accent-rgb), 0.06);
		border-color: var(--accent);
	}

	.browse-all-btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.linked-selects {
		width: 100%;
	}

	.linked-selects > :global(.custom-select-container) {
		width: 100%;
	}
</style>
