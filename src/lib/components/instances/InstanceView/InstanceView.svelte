<script lang="ts">
	import { InstState, type InstanceDto } from "$lib/types/types";
	import { launchInstance } from "$lib/api/cubicApi";
	import { t } from "$lib/i18n";
	import { fly } from "svelte/transition";
	import { killInst } from "$lib/api/launcherService";
	import { isVersionDownloading } from "$lib/state/downloadState.svelte";
	import InstanceHeader from "./InstanceHeader.svelte";
	import GridIcon from "$lib/icons/GridIcon.svelte";
	import BoxIcon from "$lib/icons/BoxIcon.svelte";
	import ImageIcon from "$lib/icons/ImageIcon.svelte";
	import ShaderIcon from "$lib/icons/ShaderIcon.svelte";
	import ChevronRightIcon from "$lib/icons/ChevronRightIcon.svelte";

	let { selectedInstance } = $props<{ selectedInstance: InstanceDto }>();
	let activeSection = $state("detalles");
	let tabContentEl: HTMLDivElement | undefined = $state();

	$effect.pre(() => {
		if (activeSection === "detalles" && tabContentEl) {
			tabContentEl.scrollTop = 0;
		}
	});
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
		if (
			!supportsMods &&
			(activeSection === "market" || activeSection === "shaderpacks")
		) {
			activeSection = "detalles";
		}
	});

	import type MarketType from "../Market/Market.svelte";
	import type ScreenshotsTabType from "../ScreenshotsTab.svelte";

	let Market: typeof MarketType | null = $state(null);
	let ScreenshotsTab: typeof ScreenshotsTabType | null = $state(null);

	$effect(() => {
		if (
			(activeSection === "market" ||
				activeSection === "resources" ||
				activeSection === "shaderpacks") &&
			!Market
		) {
			import("../Market/Market.svelte").then((m) => (Market = m.default));
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
		bind:activeSection
		onPlay={handlePlay}
	/>

	<div class="tab-content" bind:this={tabContentEl}>
		{#if activeSection === "detalles"}
			<div
				class="details-section"
				in:fly={{ x: -200, duration: 450 }}
				out:fly={{ x: -200, duration: 400 }}
			>
				<div class="nav-card">
					<span class="nav-card-header">
						<span class="nav-card-title"
							>{t("instanceView.tabs.details")}</span
						>
					</span>
					<div class="nav-items">
						{#if supportsMods}
							<button
								type="button"
								class="nav-item priority"
								onclick={() => (activeSection = "market")}
							>
								<span class="nav-icon"
									><GridIcon size={18} /></span
								>
								<span class="nav-label"
									>{t("instanceView.tabs.market")}</span
								>
								<span class="nav-chevron"
									><ChevronRightIcon size={14} /></span
								>
							</button>
						{/if}
						<button
							type="button"
							class="nav-item priority"
							onclick={() => (activeSection = "resources")}
						>
							<span class="nav-icon"><BoxIcon size={18} /></span>
							<span class="nav-label"
								>{t("instanceView.tabs.resources")}</span
							>
							<span class="nav-chevron"
								><ChevronRightIcon size={14} /></span
							>
						</button>
						<button
							type="button"
							class="nav-item"
							class:secondary={supportsMods}
							onclick={() => (activeSection = "screenshots")}
						>
							<span class="nav-icon"><ImageIcon size={18} /></span
							>
							<span class="nav-label"
								>{t("instanceView.tabs.screenshots")}</span
							>
							<span class="nav-chevron"
								><ChevronRightIcon size={14} /></span
							>
						</button>
						{#if supportsShaders}
							<button
								type="button"
								class="nav-item secondary"
								onclick={() => (activeSection = "shaderpacks")}
							>
								<span class="nav-icon"
									><ShaderIcon size={18} /></span
								>
								<span class="nav-label"
									>{t("instanceView.tabs.shaderpacks")}</span
								>
								<span class="nav-chevron"
									><ChevronRightIcon size={14} /></span
								>
							</button>
						{/if}
					</div>
				</div>
			</div>
		{:else}
			<div
				class="subview-section"
				in:fly={{ x: 200, duration: 450 }}
				out:fly={{ x: 200, duration: 400 }}
			>
				{#if activeSection === "market"}
					{#key selectedInstance.uuid}
						{#if Market}
							<Market instance={selectedInstance} />
						{/if}
					{/key}
				{:else if activeSection === "resources"}
					{#key selectedInstance.uuid}
						{#if Market}
							<Market
								instance={selectedInstance}
								contentType="resourcepacks"
							/>
						{/if}
					{/key}
				{:else if activeSection === "shaderpacks"}
					{#key selectedInstance.uuid}
						{#if Market}
							<Market
								instance={selectedInstance}
								contentType="shaderpacks"
							/>
						{/if}
					{/key}
				{:else if activeSection === "screenshots"}
					{#if ScreenshotsTab}
						<ScreenshotsTab instance={selectedInstance} />
					{/if}
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

	.tab-content {
		flex: 1;
		padding: 24px;
		overflow-y: auto;
		scrollbar-gutter: stable;
		position: relative;
	}

	.nav-card {
		position: absolute;
		bottom: 24px;
		left: 24px;
		width: 340px;
		max-width: calc(100% - 48px);
		border-radius: var(--border-radius);
		background: var(--bg-card);
		border: 1px solid var(--border);
		overflow: hidden;
		transition: box-shadow 0.6s cubic-bezier(0.25, 0.1, 0.25, 1);
	}

	.nav-card:hover {
		box-shadow: 0 4px 24px rgba(0, 0, 0, 0.2);
	}

	.nav-card-header {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 9px 14px;
		color: var(--text-tertiary);
		font-family: inherit;
		font-size: 0.72rem;
		font-weight: 600;
		letter-spacing: 0.3px;
		text-transform: uppercase;
		user-select: none;
	}

	.nav-card-title {
		flex: 1;
	}

	.nav-items {
		overflow: hidden;
		max-height: 82px;
		transition: max-height 0.6s cubic-bezier(0.25, 0.1, 0.25, 1);
	}

	.nav-card:hover .nav-items {
		max-height: 300px;
	}

	.nav-item {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 11px 14px;
		background: transparent;
		border: none;
		border-top: 1px solid var(--border);
		color: var(--text-secondary);
		font-family: inherit;
		font-size: 0.82rem;
		font-weight: 600;
		cursor: pointer;
		transition:
			background 0.12s ease,
			color 0.12s ease;
		text-align: left;
		width: 100%;
	}

	.nav-item:first-child {
		border-top: none;
	}

	.nav-item:hover {
		background: color-mix(
			in srgb,
			var(--bg-sidebar) 100%,
			var(--text-primary) 2%
		);
		color: var(--text-primary);
	}

	.nav-icon {
		display: flex;
		flex-shrink: 0;
		opacity: 0.7;
	}

	.nav-item:hover .nav-icon {
		opacity: 1;
	}

	.nav-label {
		flex: 1;
		min-width: 0;
	}

	.nav-chevron {
		display: flex;
		flex-shrink: 0;
		opacity: 0.4;
		transition:
			transform 0.15s ease,
			opacity 0.15s ease;
	}

	.nav-item:hover .nav-chevron {
		opacity: 0.8;
		transform: translateX(2px);
	}

	@media (max-width: 700px) {
		.tab-content {
			padding: 16px;
		}
		.nav-card {
			bottom: 16px;
			left: 16px;
			max-width: calc(100% - 32px);
		}
	}

	@media (max-width: 400px) {
		.tab-content {
			padding: 12px;
		}
		.nav-card {
			bottom: 12px;
			left: 12px;
			max-width: calc(100% - 24px);
		}
	}
</style>
