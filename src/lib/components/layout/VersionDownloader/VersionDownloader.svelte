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
		getQuiltVersions,
		downloadQuilt,
		refreshAvailableVersions,
		refreshForgeVersions,
		refreshQuiltVersions,
		getDownloadQueue,
	} from "$lib/api/cubicApi";
	import type {
		MinecraftVersion,
		FabricGameVersion,
		ForgeGameVersion,
		AppEvent,
	} from "$lib/types/types";
	import { listen } from "@tauri-apps/api/event";

	import VirtualList from "../VirtualList.svelte";
	import { launcherStore } from "$lib/state/state.svelte";
	import { t } from "$lib/i18n";
	import { SvelteSet } from "svelte/reactivity";
	import VersionDownloaderHeader from "./VersionDownloaderHeader.svelte";
	import VersionDownloaderTabs from "./VersionDownloaderTabs.svelte";
	import VersionDownloaderFilters from "./VersionDownloaderFilters.svelte";
	import VersionDownloaderItem from "./VersionDownloaderItem.svelte";

	interface Props {
		onclose?: () => void;
	}

	let { onclose }: Props = $props();

	let loading = $state(true);
	let manifest = $state<MinecraftVersion[] | null>(null);
	let fabricManifest = $state<FabricGameVersion[]>([]);
	let forgeManifest = $state<ForgeGameVersion[]>([]);
	let quiltManifest = $state<FabricGameVersion[]>([]);
	let installedVanilla = $state(new Set<string>());
	let installedFabric = $state(new Set<string>());
	let installedForge = $state(new Set<string>());
	let installedQuilt = $state(new Set<string>());
	let downloadingVersions = new SvelteSet<string>();

	function gameVersionFromLoaderId(versionId: string): string | null {
		const fabric = versionId.match(/^fabric-loader-[\d.]+-(.+)$/);
		if (fabric) return fabric[1];
		const quilt = versionId.match(/^quilt-loader-[\d.]+-(.+)$/);
		if (quilt) return quilt[1];
		return null;
	}

	function markDownloading(versionId: string) {
		downloadingVersions.add(versionId);
		const game = gameVersionFromLoaderId(versionId);
		if (game) downloadingVersions.add(game);
	}

	function unmarkDownloading(versionId: string) {
		downloadingVersions.delete(versionId);
		const game = gameVersionFromLoaderId(versionId);
		if (game) downloadingVersions.delete(game);
	}
	let filter = $state("release");
	let rawSearch = $state("");
	let search = $state("");
	$effect(() => {
		const timer = setTimeout(() => {
			search = rawSearch;
		}, 150);
		return () => clearTimeout(timer);
	});
	let installStatusFilter = $state("all");
	let majorVersionFilter = $state("all");
	let fabricStabilityFilter = $state("stable");

	let loadingMojang = $state(false);
	let loadingFabric = $state(false);
	let loadingForge = $state(false);
	let loadingQuilt = $state(false);
	let refreshing = $state(false);

	function refreshCurrentSource() {
		if (filter === "fabric") refreshFabric();
		else if (filter === "forge") refreshForge();
		else if (filter === "quilt") refreshQuilt();
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

	async function refreshQuilt() {
		refreshing = true;
		quiltManifest = await refreshQuiltVersions();
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

	async function loadQuilt() {
		if (quiltManifest.length > 0 || loadingQuilt) return;
		loadingQuilt = true;
		quiltManifest = await getQuiltVersions();
		loadingQuilt = false;
	}

	onMount(() => {
		getInstalledVersions().then((raw) => {
			const { vanilla, fabric, forge, quilt } =
				getInstalledMcVersions(raw);
			installedVanilla = vanilla;
			installedFabric = fabric;
			installedForge = forge;
			installedQuilt = quilt;
			loading = false;
		});

		getDownloadQueue().then((queue) => {
			for (const item of queue) {
				if (item.status !== "done") {
					markDownloading(item.version);
				}
			}
		});

		const unlisten = listen<AppEvent>("app-event", (event) => {
			const p = event.payload;
			if (p.type === "DEnqueue") {
				markDownloading(p.data.version);
			} else if (p.type === "DFinish") {
				unmarkDownloading(p.data.version);
				getInstalledVersions().then((raw) => {
					const { vanilla, fabric, forge, quilt } =
						getInstalledMcVersions(raw);
					installedVanilla = vanilla;
					installedFabric = fabric;
					installedForge = forge;
					installedQuilt = quilt;
				});
			} else if (p.type === "DError") {
				unmarkDownloading(p.data.version);
			}
		});

		return () => {
			unlisten.then((u) => u());
		};
	});

	$effect(() => {
		if (filter === "fabric") loadFabric();
		else if (filter === "forge") loadForge();
		else if (filter === "quilt") loadQuilt();
		else loadMojang();
	});

	const isCurrentManifestLoading = $derived.by(() => {
		if (filter === "fabric") return loadingFabric;
		if (filter === "forge") return loadingForge;
		if (filter === "quilt") return loadingQuilt;
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

		const source =
			filter === "fabric"
				? fabricManifest
				: filter === "quilt"
					? quiltManifest
					: manifest;
		if (!source) return [];
		const versions = new SvelteSet<string>();
		source.forEach((v: MinecraftVersion | FabricGameVersion) => {
			const vid =
				filter === "fabric" || filter === "quilt"
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

					if (installStatusFilter === "installed" && !isInstalled)
						return false;
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
					for (
						let i = 0;
						i < Math.max(aParts.length, bParts.length);
						i++
					) {
						const aVal = aParts[i] ?? 0;
						const bVal = bParts[i] ?? 0;
						if (aVal !== bVal) return bVal - aVal;
					}
					return b.forge_version.localeCompare(
						a.forge_version,
						undefined,
						{ numeric: true },
					);
				});
		}

		const source =
			filter === "fabric"
				? fabricManifest
				: filter === "quilt"
					? quiltManifest
					: manifest;
		return (
			source?.filter((v: MinecraftVersion | FabricGameVersion) => {
				const versionId =
					filter === "fabric" || filter === "quilt"
						? (v as FabricGameVersion).version
						: (v as MinecraftVersion).id;

				const isInstalled =
					filter === "fabric"
						? installedFabric.has(versionId)
						: filter === "quilt"
							? installedQuilt.has(versionId)
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

				if (filter === "fabric" || filter === "quilt") {
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
					filter === "quilt" ||
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

	const displayVersions = $derived.by(() => {
		const isFabricOrQuilt = filter === "fabric" || filter === "quilt";
		return filteredVersions.map((v) => {
			const id = isFabricOrQuilt
				? (v as FabricGameVersion).version
				: (v as MinecraftVersion).id ??
					(v as ForgeGameVersion).version_id;
			const version = isFabricOrQuilt
				? (v as FabricGameVersion).version
				: (v as MinecraftVersion).id ??
					(v as ForgeGameVersion).version_id;
			return {
				id,
				version,
				game_version: (v as ForgeGameVersion).game_version ?? "",
				forge_version: (v as ForgeGameVersion).forge_version ?? "",
				type: (v as MinecraftVersion).type ?? "",
				stable: (v as FabricGameVersion).stable ?? false,
				releaseTime: (v as MinecraftVersion).releaseTime ?? "",
				isInstalled:
					filter === "fabric"
						? installedFabric.has(id)
						: filter === "quilt"
							? installedQuilt.has(id)
							: filter === "forge"
								? installedForge.has(id)
								: installedVanilla.has(id),
				isDownloading: downloadingVersions.has(id),
			};
		});
	});

	$effect(() => {
		if (!launcherStore.settings.show_snapshots && filter === "snapshot") {
			filter = "release";
		}
		if (!launcherStore.settings.show_alpha && filter === "alpha") {
			filter = "release";
		}
	});

	async function handleDownload(
		versionId: string,
		gameVersion?: string,
		forgeVersion?: string,
	) {
		if (filter === "fabric") {
			const queued = await downloadFabric(versionId);
			if (queued) markDownloading(queued);
		} else if (filter === "quilt") {
			const queued = await downloadQuilt(versionId);
			if (queued) markDownloading(queued);
		} else if (filter === "forge" && gameVersion && forgeVersion) {
			const queued = `${gameVersion}-forge-${forgeVersion}`;
			await downloadForge(gameVersion, forgeVersion);
			markDownloading(queued);
		} else {
			await addToQueue(versionId);
			markDownloading(versionId);
		}
	}
</script>

<div class="qm-root">
	<VersionDownloaderHeader
		{onclose}
		onrefresh={refreshCurrentSource}
		{refreshing}
	/>

	<VersionDownloaderTabs
		bind:filter
		showSnapshots={launcherStore.settings.show_snapshots}
		showAlpha={launcherStore.settings.show_alpha}
	/>

	<VersionDownloaderFilters
		bind:search={rawSearch}
		bind:installStatusFilter
		bind:majorVersionFilter
		bind:fabricStabilityFilter
		{majorVersionOptions}
		{filter}
	/>

	{#if filter === "forge"}
		<div
			style="padding: 0 20px 8px; font-size: 0.75rem; color: var(--text-muted);"
		>
			{t("versionDownloader.forgeJavaHint")}
		</div>
	{/if}

	<div class="qm-scroll" style="padding: 0;">
		{#if loading || isCurrentManifestLoading}
			<div class="qm-empty-state">{t("versionDownloader.loading")}</div>
		{:else if displayVersions.length === 0}
			<div class="qm-empty-state">{t("versionDownloader.notFound")}</div>
		{:else}
			<VirtualList items={displayVersions} itemHeight={66} padding={20}>
				{#snippet children(version, _index)}
					<div
						class="virtual-item-container"
						style="padding: 0 20px;"
					>
						<VersionDownloaderItem
							{version}
							{filter}
							isInstalled={version.isInstalled}
							isDownloading={version.isDownloading}
							ondownload={() =>
								handleDownload(
									version.id,
									version.game_version || undefined,
									version.forge_version || undefined,
								)}
						/>
					</div>
				{/snippet}
			</VirtualList>
		{/if}
	</div>

	<div class="qm-footer">
		<span class="qm-version"
			>Source: {filter === "fabric"
				? "Fabric Meta"
				: filter === "quilt"
					? "Quilt Meta"
					: filter === "forge"
						? "Maven (minecraftforge.net)"
						: "Mojang Manifest"}</span
		>
	</div>
</div>
