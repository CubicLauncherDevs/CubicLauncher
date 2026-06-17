<script lang="ts">
	import { onMount } from "svelte";
	import {
		getAvailableVersions,
		addToQueue,
		getInstalledVersions,
		getInstalledMcVersions,
		getFabricVersions,
		downloadFabric,
		getForgeVersions,
		downloadForge,
		refreshAvailableVersions,
		refreshForgeVersions,
		getDownloadQueue,
	} from "$lib/api/cubicApi";
	import type {
		MinecraftVersion,
		FabricGameVersion,
		ForgeGameVersion,
		AppEvent,
	} from "$lib/types/types";
	import { listen } from "@tauri-apps/api/event";

	import VirtualList from "./VirtualList.svelte";
	import Select from "./Select.svelte";
	import { launcherStore } from "$lib/state/state.svelte";
	import { t } from "$lib/i18n";
	import { SvelteSet } from "svelte/reactivity";
	import CheckIcon from "$lib/icons/CheckIcon.svelte";

	const dateFmt = new Intl.DateTimeFormat(undefined, { dateStyle: "medium" });

	interface Props {
		onclose?: () => void;
	}

	let { onclose }: Props = $props();

	let loading = $state(true);
	let manifest = $state<MinecraftVersion[] | null>(null);
	let fabricManifest = $state<FabricGameVersion[]>([]);
	let forgeManifest = $state<ForgeGameVersion[]>([]);
	let installedVanilla = $state(new Set<string>());
	let installedFabric = $state(new Set<string>());
	let installedForge = $state(new Set<string>());
	let downloadingVersions = new SvelteSet<string>();
	let filter = $state("release");
	let search = $state("");
	let installStatusFilter = $state("all");
	let majorVersionFilter = $state("all");
	let fabricStabilityFilter = $state("stable");

	let loadingMojang = $state(false);
	let loadingFabric = $state(false);
	let loadingForge = $state(false);
	let refreshing = $state(false);

	function refreshCurrentSource() {
		if (filter === "fabric") refreshFabric();
		else if (filter === "forge") refreshForge();
		else refreshMojang();
	}

	async function refreshMojang() {
		refreshing = true;
		manifest = await refreshAvailableVersions();
		refreshing = false;
	}

	async function refreshFabric() {
		refreshing = true;
		fabricManifest = await getFabricVersions();
		refreshing = false;
	}

	async function refreshForge() {
		refreshing = true;
		forgeManifest = await refreshForgeVersions();
		refreshing = false;
	}

	async function loadMojang() {
		if (manifest || loadingMojang) return;
		loadingMojang = true;
		manifest = await getAvailableVersions();
		loadingMojang = false;
	}

	async function loadFabric() {
		if (fabricManifest.length > 0 || loadingFabric) return;
		loadingFabric = true;
		fabricManifest = await getFabricVersions();
		loadingFabric = false;
	}

	async function loadForge() {
		if (forgeManifest.length > 0 || loadingForge) return;
		loadingForge = true;
		forgeManifest = await getForgeVersions();
		loadingForge = false;
	}

	onMount(() => {
		getInstalledVersions().then((raw) => {
			const { vanilla, fabric, forge } = getInstalledMcVersions(raw);
			installedVanilla = vanilla;
			installedFabric = fabric;
			installedForge = forge;
			loading = false;
		});

		getDownloadQueue().then((queue) => {
			for (const item of queue) {
				if (item.status !== "done") {
					downloadingVersions.add(item.version);
				}
			}
		});

		const unlisten = listen<AppEvent>("app-event", (event) => {
			const p = event.payload;
			if (p.type === "DEnqueue") {
				downloadingVersions.add(p.data.version);
			} else if (p.type === "DFinish") {
				downloadingVersions.delete(p.data.version);
				getInstalledVersions().then((raw) => {
					const { vanilla, fabric, forge } = getInstalledMcVersions(raw);
					installedVanilla = vanilla;
					installedFabric = fabric;
					installedForge = forge;
				});
			} else if (p.type === "DError") {
				downloadingVersions.delete(p.data.version);
			}
		});

		return () => {
			unlisten.then((u) => u());
		};
	});

	$effect(() => {
		if (filter === "fabric") loadFabric();
		else if (filter === "forge") loadForge();
		else loadMojang();
	});

	const isCurrentManifestLoading = $derived.by(() => {
		if (filter === "fabric") return loadingFabric;
		if (filter === "forge") return loadingForge;
		return loadingMojang;
	});

	const availableMajorVersions = $derived.by(() => {
		if (filter === "forge") {
			const versions = new SvelteSet<string>();
			forgeManifest.forEach((v) => {
				const match = v.game_version.match(/^1\.\d+/);
				if (match) versions.add(match[0]);
			});
			return Array.from(versions).sort((a, b) => {
				const aNum = parseInt(a.split(".")[1] || "0");
				const bNum = parseInt(b.split(".")[1] || "0");
				return bNum - aNum;
			});
		}

		const source = filter === "fabric" ? fabricManifest : manifest;
		if (!source) return [];
		const versions = new SvelteSet<string>();
		source.forEach((v: MinecraftVersion | FabricGameVersion) => {
			const vid =
				filter === "fabric"
					? (v as FabricGameVersion).version
					: (v as MinecraftVersion).id;
			const match = vid.match(/^1\.\d+/);
			if (match) {
				versions.add(match[0]);
			}
		});
		return Array.from(versions).sort((a, b) => {
			const aNum = parseInt(a.split(".")[1] || "0");
			const bNum = parseInt(b.split(".")[1] || "0");
			return bNum - aNum;
		});
	});

	const majorVersionOptions = $derived([
		{ value: "all", label: t("versionDownloader.filters.all") },
		...availableMajorVersions.map((v) => ({ value: v, label: v })),
	]);

	const filteredVersions = $derived.by(() => {
		if (filter === "forge") {
			return forgeManifest
				.filter((v) => {
					const versionId = v.version_id;
					const isInstalled = installedForge.has(versionId);

					if (installStatusFilter === "installed" && !isInstalled) return false;
					if (installStatusFilter === "not_installed" && isInstalled)
						return false;

					if (
						majorVersionFilter !== "all" &&
						!v.game_version.startsWith(majorVersionFilter)
					)
						return false;

					const matchesSearch = versionId
						.toLowerCase()
						.includes(search.toLowerCase());
					return matchesSearch;
				})
				.sort((a, b) => {
					const aParts = a.game_version.split(".").map(Number);
					const bParts = b.game_version.split(".").map(Number);
					for (let i = 0; i < Math.max(aParts.length, bParts.length); i++) {
						const aVal = aParts[i] ?? 0;
						const bVal = bParts[i] ?? 0;
						if (aVal !== bVal) return bVal - aVal;
					}
					return b.forge_version.localeCompare(a.forge_version, undefined, {
						numeric: true,
					});
				});
		}

		const source = filter === "fabric" ? fabricManifest : manifest;
		return (
			source?.filter((v: MinecraftVersion | FabricGameVersion) => {
				const versionId =
					filter === "fabric"
						? (v as FabricGameVersion).version
						: (v as MinecraftVersion).id;

				const isInstalled =
					filter === "fabric"
						? installedFabric.has(versionId)
						: installedVanilla.has(versionId);

				if (installStatusFilter === "installed" && !isInstalled)
					return false;
				if (installStatusFilter === "not_installed" && isInstalled)
					return false;

				if (
					majorVersionFilter !== "all" &&
					!versionId.startsWith(majorVersionFilter)
				)
					return false;

				if (filter === "fabric") {
					const fv = v as FabricGameVersion;
					if (fabricStabilityFilter === "stable" && !fv.stable)
						return false;
					if (fabricStabilityFilter === "unstable" && fv.stable)
						return false;
				} else {
					const mv = v as MinecraftVersion;
					if (
						!launcherStore.settings.show_snapshots &&
						mv.type === "snapshot"
					)
						return false;
					if (
						!launcherStore.settings.show_alpha &&
						(mv.type === "old_alpha" || mv.type === "old_beta")
					)
						return false;
				}

				const matchesFilter =
					filter === "fabric" ||
					(v as MinecraftVersion).type === filter ||
					(filter === "alpha" &&
						((v as MinecraftVersion).type === "old_alpha" ||
							(v as MinecraftVersion).type === "old_beta"));

				const matchesSearch = versionId
					.toLowerCase()
					.includes(search.toLowerCase());
				return matchesFilter && matchesSearch;
			}) || []
		);
	});

	const displayVersions = $derived(
		filteredVersions.map((v) => ({
			id: (v as MinecraftVersion).id ?? (v as FabricGameVersion).version ?? (v as ForgeGameVersion).version_id,
			version:
				(v as FabricGameVersion).version ?? (v as MinecraftVersion).id ?? (v as ForgeGameVersion).version_id,
			game_version: (v as ForgeGameVersion).game_version ?? "",
			forge_version: (v as ForgeGameVersion).forge_version ?? "",
			type: (v as MinecraftVersion).type ?? "",
			stable: (v as FabricGameVersion).stable ?? false,
			releaseTime: (v as MinecraftVersion).releaseTime ?? "",
		})),
	);

	$effect(() => {
		if (!launcherStore.settings.show_snapshots && filter === "snapshot") {
			filter = "release";
		}
		if (!launcherStore.settings.show_alpha && filter === "alpha") {
			filter = "release";
		}
	});

	async function handleDownload(versionId: string, gameVersion?: string, forgeVersion?: string) {
		if (filter === "fabric") {
			await downloadFabric(versionId);
		} else if (filter === "forge" && gameVersion && forgeVersion) {
			await downloadForge(gameVersion, forgeVersion);
		} else {
			await addToQueue(versionId);
		}

		const raw = await getInstalledVersions();
		const { vanilla, fabric, forge } = getInstalledMcVersions(raw);
		installedVanilla = vanilla;
		installedFabric = fabric;
		installedForge = forge;
	}
</script>

<div class="qm-root">
	<div class="qm-header">
		<span class="qm-label">{t("versionDownloader.title")}</span>
		<div style="display: flex; align-items: center; gap: 8px;">
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
			<button type="button" class="qm-close-btn" onclick={onclose}
				>✕</button
			>
		</div>
	</div>

	<div class="qm-tabs">
		<button
			type="button"
			class="qm-tab-btn"
			class:active={filter === "release"}
			onclick={() => (filter = "release")}
		>
			<span class="qm-tab-label"
				>{t("versionDownloader.tabs.releases")}</span
			>
		</button>
		{#if launcherStore.settings.show_snapshots}
			<button
				type="button"
				class="qm-tab-btn"
				class:active={filter === "snapshot"}
				onclick={() => (filter = "snapshot")}
			>
				<span class="qm-tab-label"
					>{t("versionDownloader.tabs.snapshots")}</span
				>
			</button>
		{/if}
		{#if launcherStore.settings.show_alpha}
			<button
				type="button"
				class="qm-tab-btn"
				class:active={filter === "alpha"}
				onclick={() => (filter = "alpha")}
			>
				<span class="qm-tab-label"
					>{t("versionDownloader.tabs.alphas")}</span
				>
			</button>
		{/if}
		<button
			type="button"
			class="qm-tab-btn"
			class:active={filter === "fabric"}
			onclick={() => (filter = "fabric")}
		>
			<span class="qm-tab-label"
				>{t("versionDownloader.tabs.fabric")}</span
			>
		</button>
		<button
			type="button"
			class="qm-tab-btn"
			class:active={filter === "forge"}
			onclick={() => (filter = "forge")}
		>
			<span class="qm-tab-label"
				>{t("versionDownloader.tabs.forge")}</span
			>
		</button>
	</div>

	<div
		class="qm-search-container"
		style="padding: 10px 20px; display: flex; flex-direction: column; gap: 10px;"
	>
		<input
			type="text"
			placeholder={t("versionDownloader.searchPlaceholder")}
			bind:value={search}
			style="width: 100%; background: var(--bg-input); border: 1px solid var(--border-color); color: var(--text-primary); padding: 8px 12px; border-radius: 8px; font-size: 0.85rem;"
		/>
		<div
			class="qm-filters-grid"
			style="display: grid; grid-template-columns: repeat(auto-fit, minmax(180px, 1fr)); gap: 16px; margin-top: 4px; padding-bottom: 8px;"
		>
			<Select
				label={t("versionDownloader.filters.installStatus")}
				options={[
					{ value: "all", label: t("versionDownloader.filters.all") },
					{
						value: "installed",
						label: t("versionDownloader.filters.installedOnly"),
					},
					{
						value: "not_installed",
						label: t("versionDownloader.filters.notInstalledOnly"),
					},
				]}
				bind:value={installStatusFilter}
			/>

			<Select
				label={t("versionDownloader.filters.majorVersion")}
				options={majorVersionOptions}
				bind:value={majorVersionFilter}
			/>

			{#if filter === "fabric"}
				<Select
					label={t("versionDownloader.filters.fabricStability")}
					options={[
						{
							value: "all",
							label: t("versionDownloader.filters.all"),
						},
						{
							value: "stable",
							label: t("versionDownloader.filters.stableOnly"),
						},
						{
							value: "unstable",
							label: t("versionDownloader.filters.unstableOnly"),
						},
					]}
					bind:value={fabricStabilityFilter}
				/>
			{/if}
		</div>
	</div>

	<div class="qm-scroll" style="padding: 0;">
		{#if loading || isCurrentManifestLoading}
			<div class="qm-empty-state">{t("versionDownloader.loading")}</div>
		{:else if displayVersions.length === 0}
			<div class="qm-empty-state">{t("versionDownloader.notFound")}</div>
		{:else}
			<VirtualList items={displayVersions} itemHeight={66} padding={20}>
				{#snippet children(version, _index)}
						{@const isInstalled =
						filter === "fabric"
							? installedFabric.has(version.version)
							: filter === "forge"
								? installedForge.has(version.id)
								: installedVanilla.has(version.id)}
					<div
						class="virtual-item-container"
						style="padding: 0 20px;"
					>
						<div
							class="version-item"
							style="display: flex; align-items: center; justify-content: space-between; padding: 12px; background: var(--bg-card); border: 1px solid var(--border-color); border-radius: 8px; height: 58px;"
						>
							<div class="version-info">
								<div
									style="display: flex; align-items: center; gap: 8px;"
								>
									<div
										style="font-weight: 600; font-size: 0.9rem;"
									>
									{filter === "fabric"
										? version.version
										: version.id}
									</div>
									{#if isInstalled}
										<span class="inst-badge"
											>{t(
												"versionDownloader.installedTag",
											)}</span
										>
									{/if}
								</div>
								<div
									style="font-size: 0.7rem; color: var(--text-muted); text-transform: uppercase; letter-spacing: 0.5px;"
								>
									{#if filter === "fabric"}
										Fabric Meta • {version.stable
											? "STABLE"
											: "UNSTABLE"}
									{:else if filter === "forge"}
										Forge • MC {version.game_version}
									{:else}
										{version.type} • {dateFmt.format(new Date(
											version.releaseTime,
										))}
									{/if}
								</div>
							</div>

							{#if isInstalled}
								<div class="inst-icon">
									<CheckIcon size={10} />
								</div>
							{:else if downloadingVersions.has(
								filter === "fabric"
									? version.version
									: version.id,
							)}
								<button
									type="button"
									class="download-btn"
									class:downloading={true}
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
										handleDownload(
											filter === "fabric"
												? version.version
												: version.id,
											version.game_version || undefined,
											version.forge_version || undefined,
										)}
								>
									{t("versionDownloader.downloadBtn")}
								</button>
							{/if}
						</div>
					</div>
				{/snippet}
			</VirtualList>
		{/if}
	</div>

	<div class="qm-footer">
		<span class="qm-version"
			>Source: {filter === "fabric"
				? "Fabric Meta"
				: filter === "forge"
					? "Maven (minecraftforge.net)"
					: "Mojang Manifest"}</span
		>
	</div>
</div>

<style>
	@keyframes spin {
		from {
			transform: rotate(0deg);
		}
		to {
			transform: rotate(360deg);
		}
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
	
	}

	.download-btn:hover {
		opacity: 0.9;
	}

	.download-btn.downloading {
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

	.inst-badge {
		font-size: 0.5rem;
		background: rgba(var(--color-success-rgb), 0.1);
		color: var(--color-success);
		padding: 1px 5px;
		border-radius: 3px;
		font-weight: 700;
		text-transform: uppercase;
		border: 1px solid rgba(var(--color-success-rgb), 0.2);
		letter-spacing: 0.3px;
	}

	.inst-icon {
		color: var(--color-success);
		padding: 4px 8px;
		display: flex;
		align-items: center;
	}
</style>
