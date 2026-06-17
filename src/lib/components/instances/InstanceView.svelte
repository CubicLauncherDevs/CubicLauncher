<script lang="ts">
	import { invoke, convertFileSrc } from "@tauri-apps/api/core";
	import { InstState, type InstanceDto } from "$lib/types/types";
	import InstanceDetails from "./InstanceDetails.svelte";
	import StatusLog from "./StatusLog.svelte";
	import { launchInstance } from "$lib/api/cubicApi";
	import { t } from "$lib/i18n";
	import { killInst } from "$lib/api/launcherService";
	import { slide } from "svelte/transition";

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

	$effect(() => {
		if (!supportsMods && activeTab === "mods") {
			activeTab = "detalles";
		}
	});

	// Lazy loaded components — loaded on first tab switch
	// eslint-disable-next-line @typescript-eslint/consistent-type-imports
	let ModsRow: typeof import("./ModsRow.svelte").default | null = $state(null);
	// eslint-disable-next-line @typescript-eslint/consistent-type-imports
	let DownloadMods: typeof import("./DownloadMods.svelte").default | null = $state(null);
	// eslint-disable-next-line @typescript-eslint/consistent-type-imports
	let ResourcePacksTab: typeof import("./ResourcePacksTab.svelte").default | null = $state(null);
	// eslint-disable-next-line @typescript-eslint/consistent-type-imports
	let ScreenshotsTab: typeof import("./ScreenshotsTab.svelte").default | null = $state(null);

	$effect(() => {
		if (activeTab === "mods" && !ModsRow) {
			import("./ModsRow.svelte").then((m) => (ModsRow = m.default));
		} else if (activeTab === "download_mods" && !DownloadMods) {
			import("./DownloadMods.svelte").then((m) => (DownloadMods = m.default));
		} else if (activeTab === "resources" && !ResourcePacksTab) {
			import("./ResourcePacksTab.svelte").then((m) => (ResourcePacksTab = m.default));
		} else if (activeTab === "screenshots" && !ScreenshotsTab) {
			import("./ScreenshotsTab.svelte").then((m) => (ScreenshotsTab = m.default));
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
		// Tracking both selectedInstance.uuid and bannerVersion ensures we re-fetch
		// whenever the instance changes OR when the banner is manually updated/reset.
		void bannerVersion;
		fetchScreenshot();
	});
	const formatter = $derived(
		new Intl.DateTimeFormat(t("id"), {
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
</script>

<div class="instance-view">
	{#if showPicker}
		<div
			class="screenshot-picker-overlay"
			role="button"
			tabindex="0"
			onclick={() => (showPicker = false)}
			onkeydown={(e) => e.key === "Escape" && (showPicker = false)}
		>
			<div
				class="screenshot-picker-modal"
				role="dialog"
				aria-modal="true"
				tabindex="-1"
				onclick={(e) => e.stopPropagation()}
				onkeydown={(e) => e.stopPropagation()}
			>
				<div class="picker-header">
					<h3>{t("instanceView.pickBannerTitle")}</h3>
					<button
						type="button"
						class="close-btn"
						onclick={() => (showPicker = false)}>✕</button
					>
				</div>
				<div class="picker-content">
					{#if allScreenshots.length === 0}
						<div class="empty-picker">
							{t("instanceView.noScreenshots")}
						</div>
					{:else}
						<div class="picker-grid">
							{#each allScreenshots as path (path)}
								<button
									type="button"
									class="picker-item"
									onclick={() => selectScreenshot(path)}
								>
									<img
										src={convertFileSrc(path)}
										alt="Screenshot"
									/>
								</button>
							{/each}
						</div>
					{/if}
				</div>
			</div>
		</div>
	{/if}
	<section
		class="hero-section"
		style={screenshotUrl
			? `background-image: linear-gradient(to bottom, rgba(0, 0, 0, 0.2), rgba(0, 0, 0, 0.8)), url(${screenshotUrl})`
			: "background: linear-gradient(180deg, rgba(255, 255, 255, 0.02) 0%, rgba(0, 0, 0, 0) 100%);"}
	>
		<img
			class="instance-big-icon"
			src={selectedInstance.icon || "/images/cubic.svg"}
			alt="Icon"
		/>
		<div class="instance-title-area">
			<h2>{selectedInstance.name}</h2>
			<div class="last-played">
				{t("instanceView.lastPlayed").replace(
					"{date}",
					formatDate(selectedInstance.last_played),
				)}
			</div>
			{#if bannerState == "Started"}
				<button
					type="button"
					class="play-btn"
					onclick={() => killInst(selectedInstance.uuid)}
					>{t("instanceView.close")}</button
				>
			{:else if bannerState == "Starting"}
				<button type="button" class="play-btn" disabled
					>{t("instanceView.playBtn")}</button
				>
			{:else}
				<button
					type="button"
					class="play-btn"
					onclick={() => launchInstance(selectedInstance)}
					>{t("instanceView.playBtn")}</button
				>
			{/if}
		</div>

		<div class="banner-controls">
			<button
				type="button"
				class="banner-btn"
				onclick={pickBanner}
				title={t("instanceView.changeBannerTitle")}
			>
				<svg
					xmlns="http://www.w3.org/2000/svg"
					width="16"
					height="16"
					viewBox="0 0 24 24"
					fill="none"
					stroke="currentColor"
					stroke-width="2"
					stroke-linecap="round"
					stroke-linejoin="round"
					><path
						d="M23 19a2 2 0 0 1-2 2H3a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h4l2-3h6l2 3h4a2 2 0 0 1 2 2z"
					/><circle cx="12" cy="13" r="4" /></svg
				>
				<span>{t("instanceView.changeBanner")}</span>
			</button>
		</div>
	</section>
	{#if bannerState !== "Idle" && bannerState !== "Error"}
		<div transition:slide={{ duration: 300, easing: (t) => 1 - Math.pow(1 - t, 3) }}>
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
					<ResourcePacksTab instanceId={selectedInstance.uuid} />
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

	.hero-section {
		padding: 50px 40px;
		display: flex;
		align-items: center;
		gap: 28px;
		border-bottom: 1px solid var(--border);
		flex-shrink: 0;
		position: relative;
		background-size: cover;
		background-position: center;
		transition: all 0.5s cubic-bezier(0.4, 0, 0.2, 1);
	}

	.banner-controls {
		position: absolute;
		top: 20px;
		right: 20px;
		display: flex;
		gap: 8px;
		opacity: 0;
		transition: opacity 0.3s ease;
	}

	.hero-section:hover .banner-controls {
		opacity: 1;
	}

	.screenshot-picker-overlay {
		position: fixed;
		top: 0;
		left: 0;
		right: 0;
		bottom: 0;
		background: rgba(0, 0, 0, 0.7);
		backdrop-filter: blur(8px);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 2000;
		animation: fadeIn 0.2s ease-out;
	}

	.screenshot-picker-modal {
		background: var(--bg-sidebar);
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		width: 600px;
		max-width: 90vw;
		max-height: 80vh;
		display: flex;
		flex-direction: column;
		box-shadow: 0 20px 50px rgba(0, 0, 0, 0.5);
		animation: scaleUp 0.3s cubic-bezier(0.2, 0.8, 0.2, 1);
	}

	.picker-header {
		padding: 16px 20px;
		border-bottom: 1px solid var(--border);
		display: flex;
		justify-content: space-between;
		align-items: center;
	}

	.picker-header h3 {
		font-size: 0.9rem;
		font-weight: 700;
		color: var(--text-primary);
		text-transform: uppercase;
		letter-spacing: 1px;
	}

	.picker-header :global(.close-btn) {
		background: transparent;
		border: none;
		color: var(--text-secondary);
		font-size: 1.2rem;
		cursor: pointer;
		transition: color 0.2s;
	}

	.picker-header :global(.close-btn):hover {
		color: var(--text-primary);
	}

	.picker-content {
		flex: 1;
		overflow-y: auto;
		padding: 20px;
	}

	.picker-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(160px, 1fr));
		gap: 12px;
	}

	.picker-item {
		background: rgba(255, 255, 255, 0.02);
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		overflow: hidden;
		padding: 0;
		aspect-ratio: 16/9;
		cursor: pointer;
		transition: all 0.25s cubic-bezier(0.4, 0, 0.2, 1);
		position: relative;
	}

	.picker-item img {
		width: 100%;
		height: 100%;
		object-fit: cover;
		opacity: 0.8;
		transition: all 0.25s;
	}

	.picker-item:hover {
		border-color: var(--accent);
		box-shadow: 0 10px 20px rgba(0, 0, 0, 0.3);
	}

	.picker-item:hover img {
		opacity: 1;
	}

	.empty-picker {
		text-align: center;
		padding: 40px;
		color: var(--text-secondary);
		font-size: 0.85rem;
	}

	@keyframes fadeIn {
		from {
			opacity: 0;
		}
		to {
			opacity: 1;
		}
	}

	@keyframes scaleUp {
		from {
			opacity: 0;
			transform: scale(0.95);
		}
		to {
			opacity: 1;
			transform: scale(1);
		}
	}

	.banner-btn {
		background: rgba(0, 0, 0, 0.5);
		backdrop-filter: blur(4px);
		border: 1px solid rgba(255, 255, 255, 0.1);
		color: white;
		padding: 6px 12px;
		border-radius: var(--border-radius-sm);
		font-size: 0.7rem;
		cursor: pointer;
		display: flex;
		align-items: center;
		gap: 6px;
		transition: all 0.2s;
	}

	.banner-btn:hover {
		background: rgba(255, 255, 255, 0.1);
		border-color: rgba(255, 255, 255, 0.2);
	}

	@keyframes rotate {
		from {
			transform: rotate(0deg);
		}
		to {
			transform: rotate(-360deg);
		}
	}

	.instance-big-icon {
		width: 64px;
		height: 64px;
		border-radius: var(--border-radius-sm);
		display: flex;
		align-items: center;
		justify-content: center;
		flex-shrink: 0;
		filter: drop-shadow(0 0 10px rgba(255, 255, 255, 0.4));
	}

	.instance-title-area {
		display: flex;
		flex-direction: column;
		gap: 8px;
		text-shadow: 0 2px 10px rgba(0, 0, 0, 0.5);
	}

	.instance-title-area h2 {
		font-size: 1.5rem;
		font-weight: 800;
		letter-spacing: -0.5px;
		color: white;
	}

	.last-played {
		color: rgba(255, 255, 255, 0.8);
		font-size: 0.65rem;
		text-transform: uppercase;
		letter-spacing: 1.5px;
		font-weight: 700;
	}

	.play-btn {
		background: white;
		color: black;
		border: none;
		padding: 10px 28px;
		border-radius: var(--border-radius-sm);
		font-size: 0.8rem;
		font-weight: 800;
		cursor: pointer;
		width: fit-content;
		transition:
			background 0.2s ease,
			box-shadow 0.2s ease;
		font-family: "Cantarell", system-ui, sans-serif;
		letter-spacing: 0.5px;
		text-transform: uppercase;
		box-shadow: 0 4px 15px rgba(0, 0, 0, 0.3);
	}

	.play-btn:hover:not(:disabled) {
		background: #f0f0f0;
		box-shadow: 0 6px 20px rgba(0, 0, 0, 0.4);
	}

	.play-btn:active:not(:disabled) {
		transform: scale(0.98);
	}

	.play-btn:disabled {
		background: rgba(255, 255, 255, 0.15);
		color: rgba(255, 255, 255, 0.35);
		cursor: not-allowed;
		box-shadow: none;
		border: 1px solid rgba(255, 255, 255, 0.1);
	}

	.tabs-nav {
		display: flex;
		gap: 12px;
		padding: 0 40px;
		border-bottom: 1px solid var(--border);
		background: rgba(255, 255, 255, 0.01);
		backdrop-filter: blur(8px);
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
		.hero-section {
			padding: 40px;
		}
		.tabs-nav {
			padding: 0 30px;
		}
		.tab-content {
			padding: 28px 30px;
		}
	}

	@media (max-width: 950px) {
		.hero-section {
			padding: 30px 24px;
			gap: 20px;
		}
		.tabs-nav {
			padding: 0 24px;
		}
		.tab-content {
			padding: 24px;
		}
	}

	@media (max-width: 850px) {
		.hero-section {
			padding: 24px 20px;
			gap: 16px;
		}
		.tabs-nav {
			padding: 0 20px;
			gap: 8px;
		}
		.tab-item {
			font-size: 0.75rem;
			padding: 14px 3px;
			margin-right: 14px;
			white-space: nowrap;
			flex-shrink: 0;
		}
		.tab-content {
			padding: 20px;
		}
	}

	@media (max-width: 700px) {
		.hero-section {
			padding: 20px 16px;
			gap: 14px;
		}
		.instance-big-icon {
			width: 48px;
			height: 48px;
		}
		.instance-title-area h2 {
			font-size: 1.3rem;
		}
		.tabs-nav {
			padding: 0 16px;
			gap: 4px;
			overflow-x: auto;
		}
		:global(.tabs-nav::-webkit-scrollbar) {
			display: none;
		}
		.tab-item {
			font-size: 0.7rem;
			padding: 12px 2px;
			margin-right: 10px;
		}
		.tab-content {
			padding: 16px;
		}
		.banner-controls {
			opacity: 1;
		}
	}

	@media (max-width: 650px) {
		.hero-section {
			flex-direction: column;
			align-items: center;
			text-align: center;
			padding: 24px 16px;
		}
		.play-btn {
			margin: 0 auto;
		}
	}

	@media (max-width: 550px) {
		.hero-section {
			padding: 16px 12px;
			gap: 12px;
		}
		.instance-big-icon {
			width: 40px;
			height: 40px;
		}
		.instance-title-area h2 {
			font-size: 1.1rem;
		}
		.play-btn {
			padding: 8px 20px;
			font-size: 0.7rem;
		}
		.last-played {
			font-size: 0.58rem;
		}
		.tabs-nav {
			padding: 0 12px;
			gap: 2px;
		}
		.tab-item {
			font-size: 0.6rem;
			padding: 10px 2px;
			margin-right: 8px;
			letter-spacing: 0.5px;
		}
		.tab-content {
			padding: 12px;
		}
	}

	@media (max-width: 400px) {
		.hero-section {
			padding: 12px 8px;
			gap: 8px;
		}
		.instance-big-icon {
			width: 32px;
			height: 32px;
		}
		.instance-title-area h2 {
			font-size: 1rem;
		}
		.tabs-nav {
			padding: 0 8px;
		}
		.tab-item {
			font-size: 0.55rem;
			padding: 8px 1px;
			margin-right: 6px;
		}
		.tab-content {
			padding: 8px;
		}
	}
</style>
