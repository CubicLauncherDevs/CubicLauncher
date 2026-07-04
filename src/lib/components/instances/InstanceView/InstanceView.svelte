<script lang="ts">
	import { invoke, convertFileSrc } from "@tauri-apps/api/core";
	import { InstState, type InstanceDto } from "$lib/types/types";
	import InstanceDetails from "../InstanceDetails/InstanceDetails.svelte";
	import StatusLog from "../StatusLog.svelte";
	import { launchInstance } from "$lib/api/cubicApi";
	import { t } from "$lib/i18n";
	import { killInst } from "$lib/api/launcherService";
	import { launcherStore } from "$lib/state/state.svelte";
	import { slide } from "svelte/transition";
	import ScreenshotPicker from "./ScreenshotPicker.svelte";
	import HeroSection from "./HeroSection.svelte";

	let { selectedInstance } = $props<{ selectedInstance: InstanceDto }>();
	let activeTab = $state("detalles");
	let screenshotUrl = $state<string | null>(null);
	let allScreenshots = $state<string[]>([]);
	let showPicker = $state(false);
	let bannerVersion = $state(0);
	let bannerState = $derived.by(() => {
		if (selectedInstance.status === InstState.Started) return "Started";
		if (selectedInstance.status === InstState.Off) return "Idle";
		if (selectedInstance.status === InstState.Error) return "Error";
		if (selectedInstance.status === InstState.Starting) return "Starting";
		return "Idle";
	});
	const supportsMods = $derived(selectedInstance.loader !== "Vanilla");
	const supportsShaders = $derived(selectedInstance.loader !== "Vanilla");

	$effect(() => {
		if (!supportsMods && activeTab === "mods") {
			activeTab = "detalles";
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
		if (activeTab === "mods" && !ModsRow) {
			import("../ModsRow/ModsRow.svelte").then((m) => (ModsRow = m.default));
		} else if (activeTab === "download_mods" && !DownloadMods) {
			import("../DownloadMods/DownloadMods.svelte").then(
				(m) => (DownloadMods = m.default),
			);
		} else if (activeTab === "resources" && !ResourcePacksTab) {
			import("../ResourcePacks/ResourcePacks.svelte").then(
				(m) => (ResourcePacksTab = m.default),
			);
		} else if (activeTab === "screenshots" && !ScreenshotsTab) {
			import("../ScreenshotsTab.svelte").then(
				(m) => (ScreenshotsTab = m.default),
			);
		}
	});

	async function fetchScreenshot() {
		if (!selectedInstance) return;

		const path = await invoke<string | null>("get_instance_banner", {
			instanceId: selectedInstance.uuid,
		});
		if (path) {
			const clean = decodeURIComponent(path);
			screenshotUrl = convertFileSrc(clean);
		} else {
			screenshotUrl = null;
		}
	}

	async function pickBanner() {
		allScreenshots = await invoke<string[]>(
			"get_all_instance_screenshots",
			{
				instanceName: selectedInstance.name,
			},
		);
		showPicker = true;
	}

	async function selectScreenshot(path: string) {
		await invoke("set_instance_cover_image", {
			instanceId: selectedInstance.uuid,
			path: path,
		});
		showPicker = false;
		bannerVersion++;
	}

	$effect(() => {
		void bannerVersion;
		fetchScreenshot();
	});

	const lang = $derived(launcherStore.settings.language);
	const formatter = $derived(
		new Intl.DateTimeFormat(lang, {
			year: "numeric",
			month: "long",
			day: "2-digit",
			hour: "2-digit",
			minute: "2-digit",
		}),
	);

	function formatDate(unix_date: number): string {
		if (unix_date < 1) {
			return t("instanceView.neverPlayed");
		}
		let date = new Date(unix_date * 1000);
		return formatter.format(date);
	}

	function handlePlay() {
		if (bannerState === "Started") {
			killInst(selectedInstance.uuid);
		} else {
			launchInstance(selectedInstance);
		}
	}
</script>

<div class="instance-view">
	<ScreenshotPicker bind:showPicker {allScreenshots} onSelect={selectScreenshot} />

	<HeroSection
		instanceName={selectedInstance.name}
		instanceIcon={selectedInstance.icon}
		lastPlayedLabel={t("instanceView.lastPlayed").replace(
			"{date}",
			formatDate(selectedInstance.last_played),
		)}
		{screenshotUrl}
		{bannerState}
		onPlay={handlePlay}
		onPickBanner={pickBanner}
	/>

	{#if bannerState !== "Idle" && bannerState !== "Error"}
		<div
			transition:slide={{
				duration: 300,
				easing: (t) => 1 - Math.pow(1 - t, 3),
			}}
		>
			<StatusLog instance={selectedInstance} />
		</div>
	{/if}

	<div class="tabs-nav">
		<button
			type="button"
			class="tab-item {activeTab === 'detalles' ? 'active' : ''}"
			onclick={() => (activeTab = "detalles")}
		>
			{t("instanceView.tabs.details")}
		</button>
		<button
			type="button"
			class="tab-item {activeTab === 'mods' ? 'active' : ''}"
			onclick={() => supportsMods && (activeTab = "mods")}
			disabled={!supportsMods}
		>
			{t("instanceView.tabs.mods")}
		</button>
		<button
			type="button"
			class="tab-item {activeTab === 'download_mods' ? 'active' : ''}"
			onclick={() => supportsMods && (activeTab = "download_mods")}
			disabled={!supportsMods}
		>
			{t("instanceView.tabs.downloadMods") || "Get Mods"}
		</button>
		<button
			type="button"
			class="tab-item {activeTab === 'resources' ? 'active' : ''}"
			onclick={() => (activeTab = "resources")}
		>
			{t("instanceView.tabs.resources")}
		</button>
		<button
			type="button"
			class="tab-item {activeTab === 'screenshots' ? 'active' : ''}"
			onclick={() => (activeTab = "screenshots")}
		>
			{t("instanceView.tabs.screenshots")}
		</button>
	</div>

	<div class="tab-content">
		{#if activeTab === "detalles"}
			<div class="tab-pane">
				<InstanceDetails instance={selectedInstance} />
			</div>
		{:else if activeTab === "mods"}
			<div class="tab-pane">
				{#key selectedInstance.uuid}
					{#if ModsRow}
						<ModsRow instanceId={selectedInstance.uuid} />
					{/if}
				{/key}
			</div>
		{:else if activeTab === "download_mods"}
			<div class="tab-pane">
				{#key selectedInstance.uuid}
					{#if DownloadMods}
						<DownloadMods instance={selectedInstance} />
					{/if}
				{/key}
			</div>
		{:else if activeTab === "resources"}
			<div class="tab-pane">
				{#if ResourcePacksTab}
					<ResourcePacksTab
						instanceId={selectedInstance.uuid}
						gameVersion={selectedInstance.version}
						loader={selectedInstance.loader}
						{supportsShaders}
					/>
				{/if}
			</div>
		{:else if activeTab === "screenshots"}
			<div class="tab-pane">
				{#if ScreenshotsTab}
					<ScreenshotsTab instance={selectedInstance} />
				{/if}
			</div>
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

	.tabs-nav {
		display: flex;
		gap: 12px;
		padding: 0 40px;
		border-bottom: 1px solid var(--border);
		background: rgba(255, 255, 255, 0.01);
		backdrop-filter: blur(var(--backdrop-blur-float, 8px));
		position: sticky;
		top: 0;
		z-index: 10;
		flex-shrink: 0;
	}

	.tab-item {
		background: transparent;
		border: none;
		color: var(--text-secondary);
		padding: 16px 4px;
		margin-right: 20px;
		font-family: inherit;
		font-size: 0.85rem;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 1px;
		cursor: pointer;
		position: relative;
		transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
		opacity: 0.7;
	}

	.tab-item:hover {
		color: var(--text-primary);
		opacity: 1;
	}

	.tab-item.active {
		color: var(--accent);
		opacity: 1;
	}

	.tab-item:disabled {
		opacity: 0.3;
		cursor: not-allowed;
	}

	.tab-item::after {
		content: "";
		position: absolute;
		bottom: -1px;
		left: 0;
		right: 0;
		height: 2px;
		background: var(--accent);
		transform: scaleX(0);
		transition: transform 0.3s cubic-bezier(0.4, 0, 0.2, 1);
		border-radius: 2px 2px 0 0;
		box-shadow: 0 0 10px rgba(255, 255, 255, 0.3);
	}

	.tab-item.active::after {
		transform: scaleX(1);
	}

	.tab-content {
		flex: 1;
		padding: 32px 40px;
		overflow-y: auto;
		scrollbar-gutter: stable;
	}

	.tab-pane {
		animation: slideUpFade 0.4s cubic-bezier(0.2, 0.8, 0.2, 1);
		height: 100%;
	}

	@keyframes slideUpFade {
		from {
			opacity: 0;
			transform: translateY(15px);
		}
		to {
			opacity: 1;
			transform: translateY(0);
		}
	}

	@media (max-width: 1024px) {
		.tabs-nav { padding: 0 30px; }
		.tab-content { padding: 28px 30px; }
	}

	@media (max-width: 950px) {
		.tabs-nav { padding: 0 24px; }
		.tab-content { padding: 24px; }
	}

	@media (max-width: 850px) {
		.tabs-nav { padding: 0 20px; gap: 8px; }
		.tab-item { font-size: 0.75rem; padding: 14px 3px; margin-right: 14px; white-space: nowrap; flex-shrink: 0; }
		.tab-content { padding: 20px; }
	}

	@media (max-width: 700px) {
		.tabs-nav { padding: 0 16px; gap: 4px; overflow-x: auto; }
		:global(.tabs-nav::-webkit-scrollbar) { display: none; }
		.tab-item { font-size: 0.7rem; padding: 12px 2px; margin-right: 10px; }
		.tab-content { padding: 16px; }
	}

	@media (max-width: 550px) {
		.tabs-nav { padding: 0 12px; gap: 2px; }
		.tab-item { font-size: 0.6rem; padding: 10px 2px; margin-right: 8px; letter-spacing: 0.5px; }
		.tab-content { padding: 12px; }
	}

	@media (max-width: 400px) {
		.tabs-nav { padding: 0 8px; }
		.tab-item { font-size: 0.55rem; padding: 8px 1px; margin-right: 6px; }
		.tab-content { padding: 8px; }
	}
</style>
