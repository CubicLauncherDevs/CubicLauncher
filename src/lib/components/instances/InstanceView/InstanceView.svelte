<script lang="ts">
	import { InstState, type InstanceDto } from "$lib/types/types";
	import { launchInstance } from "$lib/api/cubicApi";
	import { t } from "$lib/i18n";
	import { killInst } from "$lib/api/launcherService";
	import { isVersionDownloading } from "$lib/state/downloadState.svelte";
	import InstanceHeader from "./InstanceHeader.svelte";

	let { selectedInstance } = $props<{ selectedInstance: InstanceDto }>();
	let activeSection = $state("detalles");
	let bannerState = $derived.by(() => {
		if (selectedInstance.status === InstState.Started) return "Started";
		if (selectedInstance.status === InstState.Off) return "Idle";
		if (selectedInstance.status === InstState.Error) return "Error";
		if (selectedInstance.status === InstState.Starting) return "Starting";
		return "Idle";
	});
	const supportsMods = $derived(selectedInstance.loader !== "Vanilla");
	const supportsShaders = $derived(selectedInstance.loader !== "Vanilla");
	const isDownloadingVersion = $derived(
		isVersionDownloading(selectedInstance.version),
	);

	$effect(() => {
		if (!supportsMods && activeSection === "mods") {
			activeSection = "detalles";
		}
	});

	import type ModsRowType from "../ModsRow/ModsRow.svelte";
	import type DownloadModsType from "../DownloadMods/DownloadMods.svelte";
	import type ResourcePacksTabType from "../ResourcePacks/ResourcePacks.svelte";
	import type ScreenshotsTabType from "../ScreenshotsTab.svelte";

	let ModsRow: typeof ModsRowType | null = $state(null);
	let DownloadMods: typeof DownloadModsType | null = $state(null);
	let ResourcePacksTab: typeof ResourcePacksTabType | null = $state(null);
	let ScreenshotsTab: typeof ScreenshotsTabType | null = $state(null);

	$effect(() => {
		if (activeSection === "mods" && !ModsRow) {
			import("../ModsRow/ModsRow.svelte").then(
				(m) => (ModsRow = m.default),
			);
		} else if (activeSection === "download_mods" && !DownloadMods) {
			import("../DownloadMods/DownloadMods.svelte").then(
				(m) => (DownloadMods = m.default),
			);
		} else if (activeSection === "resources" && !ResourcePacksTab) {
			import("../ResourcePacks/ResourcePacks.svelte").then(
				(m) => (ResourcePacksTab = m.default),
			);
		} else if (activeSection === "screenshots" && !ScreenshotsTab) {
			import("../ScreenshotsTab.svelte").then(
				(m) => (ScreenshotsTab = m.default),
			);
		}
	});

	function handlePlay() {
		if (bannerState === "Started") {
			killInst(selectedInstance.uuid);
		} else {
			launchInstance(selectedInstance);
		}
	}
</script>

<div class="instance-view">
	<InstanceHeader
		instance={selectedInstance}
		{bannerState}
		{isDownloadingVersion}
		onPlay={handlePlay}
	/>

	<div class="tab-content">
		{#if activeSection === "detalles"}
			<div class="section-links">
				<button
					type="button"
					class="section-link"
					disabled={!supportsMods}
					onclick={() => (activeSection = "mods")}
				>
					{t("instanceView.tabs.mods")}
				</button>
				<button
					type="button"
					class="section-link"
					disabled={!supportsMods}
					onclick={() => (activeSection = "download_mods")}
				>
					{t("instanceView.tabs.downloadMods") || "Get Mods"}
				</button>
				<button
					type="button"
					class="section-link"
					onclick={() => (activeSection = "resources")}
				>
					{t("instanceView.tabs.resources")}
				</button>
				<button
					type="button"
					class="section-link"
					onclick={() => (activeSection = "screenshots")}
				>
					{t("instanceView.tabs.screenshots")}
				</button>
			</div>

		{:else if activeSection === "mods"}
			<button type="button" class="back-link" onclick={() => (activeSection = "detalles")}>
				<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
					<line x1="19" y1="12" x2="5" y2="12" /><polyline points="12 19 5 12 12 5" />
				</svg>
				{t("instanceView.tabs.details")}
			</button>
			{#key selectedInstance.uuid}
				{#if ModsRow}
					<ModsRow instanceId={selectedInstance.uuid} />
				{/if}
			{/key}

		{:else if activeSection === "download_mods"}
			<button type="button" class="back-link" onclick={() => (activeSection = "detalles")}>
				<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
					<line x1="19" y1="12" x2="5" y2="12" /><polyline points="12 19 5 12 12 5" />
				</svg>
				{t("instanceView.tabs.details")}
			</button>
			{#key selectedInstance.uuid}
				{#if DownloadMods}
					<DownloadMods instance={selectedInstance} />
				{/if}
			{/key}

		{:else if activeSection === "resources"}
			<button type="button" class="back-link" onclick={() => (activeSection = "detalles")}>
				<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
					<line x1="19" y1="12" x2="5" y2="12" /><polyline points="12 19 5 12 12 5" />
				</svg>
				{t("instanceView.tabs.details")}
			</button>
			{#if ResourcePacksTab}
				<ResourcePacksTab
					instanceId={selectedInstance.uuid}
					gameVersion={selectedInstance.version}
					loader={selectedInstance.loader}
					{supportsShaders}
				/>
			{/if}

		{:else if activeSection === "screenshots"}
			<button type="button" class="back-link" onclick={() => (activeSection = "detalles")}>
				<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
					<line x1="19" y1="12" x2="5" y2="12" /><polyline points="12 19 5 12 12 5" />
				</svg>
				{t("instanceView.tabs.details")}
			</button>
			{#if ScreenshotsTab}
				<ScreenshotsTab instance={selectedInstance} />
			{/if}
		{/if}
	</div>
</div>

<style>
	.instance-view {
		position: relative;
		display: flex;
		flex-direction: column;
		height: 100%;
	}

	.tab-content {
		flex: 1;
		padding: 24px;
		overflow-y: auto;
		scrollbar-gutter: stable;
	}

	.section-links {
		display: flex;
		flex-wrap: wrap;
		gap: 6px;
		margin-top: 24px;
		padding-top: 24px;
		border-top: 1px solid var(--border);
	}

	.section-link {
		padding: 7px 16px;
		border-radius: 8px;
		background: var(--bg-card);
		border: 1px solid var(--border);
		color: var(--text-secondary);
		font-family: inherit;
		font-size: 0.78rem;
		font-weight: 600;
		cursor: pointer;
		transition: all 0.15s ease;
	}

	.section-link:hover:not(:disabled) {
		color: var(--text-primary);
		border-color: var(--text-tertiary);
	}

	.section-link:disabled {
		opacity: 0.3;
		cursor: not-allowed;
	}

	.back-link {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		background: transparent;
		border: none;
		color: var(--text-tertiary);
		font-family: inherit;
		font-size: 0.78rem;
		font-weight: 600;
		cursor: pointer;
		padding: 4px 0;
		margin-bottom: 16px;
		transition: color 0.15s;
	}

	.back-link:hover {
		color: var(--text-primary);
	}

	@media (max-width: 700px) {
		.tab-content {
			padding: 16px;
		}
	}

	@media (max-width: 400px) {
		.tab-content {
			padding: 12px;
		}
	}
</style>
