<script lang="ts">
	import { t } from "$lib/i18n";
	import { invoke } from "@tauri-apps/api/core";
	import { onMount } from "svelte";
	import { subscribeLogPreview } from "$lib/api/logStream";
	import type { InstanceDto } from "$lib/types/types";
	import { getLoaderLogo, getDisplayIconSrc } from "$lib/icons/logos";
	import Icon from "$lib/icons/Icon.svelte";
	import { animateWidth } from "$lib/utils/animateWidth";
	import { animDuration } from "$lib/utils/animations";
	import { formatPlaytime, hasPlaytime } from "$lib/utils/playtime";

	let {
		instance,
		bannerState = "Idle",
		downloadKind = null,
		downloadProgress = 0,
		activeSection = $bindable("detalles"),
		onPlay = () => {},
	}: {
		instance: InstanceDto;
		bannerState: string;
		downloadKind?: "mods" | "version" | null;
		downloadProgress?: number;
		activeSection: string;
		onPlay: () => void;
	} = $props();

	const loaderIcon = $derived(getLoaderLogo(instance.loader));
	const isDefaultIcon = $derived(!instance.icon);
	const downloadLabel = $derived(
		downloadKind === "mods"
			? t("instanceView.downloadingMods")
			: t("instanceView.downloadingVersion"),
	);
	const buttonDownloading = $derived(
		!!downloadKind &&
			bannerState !== "Started" &&
			bannerState !== "Starting",
	);
	const buttonLabel = $derived(
		bannerState === "Started"
			? t("instanceView.close")
			: buttonDownloading
				? downloadLabel
				: t("instanceView.playBtn"),
	);
	const resizeDuration = $derived(animDuration(220));

	const statusLabel = $derived(
		instance.status === "started"
			? t("instanceView.status.started")
			: instance.status === "starting"
				? t("instanceView.status.starting")
				: instance.status === "error"
					? "Error"
					: t("instanceView.status.idle"),
	);
	const statusClass = $derived(
		instance.status === "started"
			? "status-started"
			: instance.status === "starting"
				? "status-starting"
				: instance.status === "error"
					? "status-error"
					: "status-idle",
	);

	const playtimeLabel = $derived(
		formatPlaytime(instance.playtime_seconds, t),
	);

	let lastLog = $state("");
	let visible = $state(true);
	onMount(() => {
		const update = () => {
			visible = !document.hidden;
		};
		update();
		document.addEventListener("visibilitychange", update);
		return () => document.removeEventListener("visibilitychange", update);
	});

	$effect(() => {
		void instance.uuid;
		lastLog = "";
	});

	$effect(() => {
		const id = instance.uuid;
		if (
			!visible ||
			activeSection !== "detalles" ||
			(bannerState !== "Started" && bannerState !== "Starting")
		)
			return;
		return subscribeLogPreview(id, (line) => {
			lastLog = line;
		});
	});

	function openDir(subDir?: string) {
		invoke("open_instance_dir", {
			id: instance.uuid,
			subDir: subDir ?? null,
		});
	}

	function openLogs() {
		invoke("open_log_window", {
			instanceId: instance.uuid,
			instanceName: instance.name,
		});
	}
</script>

{#snippet launchButton()}
	<button
		type="button"
		class="launch-btn"
		class:downloading={buttonDownloading}
		use:animateWidth={resizeDuration}
		disabled={bannerState === "Starting" || buttonDownloading}
		onclick={onPlay}
		aria-label={buttonDownloading
			? `${buttonLabel} ${downloadProgress}%`
			: buttonLabel}
	>
		<span class="launch-label">
			<span>{buttonLabel}</span>
			{#if buttonDownloading}
				<span class="download-percent">{downloadProgress}%</span>
			{/if}
		</span>
		{#if buttonDownloading}
			<svg class="download-border" aria-hidden="true">
				<rect
					width="100%"
					height="100%"
					pathLength="100"
					stroke-dasharray={`${downloadProgress} 100`}
				/>
			</svg>
		{/if}
	</button>
{/snippet}

<header class="instance-header" class:compact={activeSection !== "detalles"}>
	<div class="header-bg"></div>
	<div class="header-body">
		<div
			class="header-content"
			class:visible={activeSection === "detalles"}
		>
			<div class="title-row">
				<div class="title-left">
					{#if isDefaultIcon}
						<div
							class="instance-icon instance-icon--accent"
							role="img"
							aria-label={instance.name}
						></div>
					{:else}
						<img
							class="instance-icon"
							src={getDisplayIconSrc(instance.icon)}
							alt={instance.name}
						/>
					{/if}
					<h1 class="instance-title">{instance.name}</h1>
				</div>
				<div class="actions-row">
					<button
						type="button"
						class="action-btn"
						onclick={() => openDir()}
					>
						<Icon name="instance:folder" size={15} />
						<span class="action-label"
							>{t("instanceView.options.folder")}</span
						>
					</button>
					<button
						type="button"
						class="action-btn"
						onclick={() => openDir("mods")}
					>
						<Icon name="instance:grid" size={15} />
						<span class="action-label"
							>{t("instanceView.tabs.mods")}</span
						>
					</button>
					<button
						type="button"
						class="action-btn"
						onclick={() => openDir("screenshots")}
					>
						<Icon name="instance:image" size={15} />
						<span class="action-label"
							>{t("instanceView.tabs.screenshots")}</span
						>
					</button>
					<button
						type="button"
						class="action-btn"
						onclick={() => openDir("resourcepacks")}
					>
						<Icon name="instance:database" size={15} />
						<span class="action-label"
							>{t("instanceView.tabs.resources")}</span
						>
					</button>
					<button
						type="button"
						class="action-btn"
						onclick={() => openLogs()}
					>
						<Icon name="instance:code" size={15} />
						<span class="action-label"
							>{t("instanceView.tabs.logs")}</span
						>
					</button>
				</div>
			</div>

			<div class="extension-row">
				<div class="extension-left">
					<span class="meta-chip">
						<img
							src={loaderIcon}
							alt={instance.loader}
							class="loader-icon"
						/>
						{instance.version}
					</span>
					<span class="meta-sep">·</span>
					<span class="inline-status {statusClass}">
						{#if bannerState === "Starting"}
							<svg
								class="status-spin"
								width="12"
								height="12"
								viewBox="0 0 24 24"
								fill="none"
								stroke="currentColor"
								stroke-width="2.5"
							>
								<circle
									cx="12"
									cy="12"
									r="10"
									stroke-dasharray="31.4"
									stroke-dashoffset="10"
								/>
							</svg>
						{:else if bannerState === "Started"}
							<Icon name="ui:check" size={12} />
						{/if}
						<span class="status-text">{statusLabel}</span>
						{#if bannerState === "Idle" || bannerState === "Error"}
							<span class="log-snippet"
								>{t("instanceView.status.offlineLog")}</span
							>
						{:else}
							<span class="log-snippet" title={lastLog}
								>{lastLog}</span
							>
						{/if}
					</span>
				</div>
				<div class="launch-area">
					{@render launchButton()}
				</div>
			</div>

			<div class="details-row">
				<div class="path-row">
					<Icon name="instance:folder" size={12} />
					<span class="path-text" title={instance.path}
						>{instance.path}</span
					>
					<button
						type="button"
						class="icon-btn"
						onclick={() => openDir()}
						title={t("instanceView.details.location")}
					>
						<Icon name="instance:external-link" size={13} />
					</button>
				</div>
				{#if hasPlaytime(instance.playtime_seconds)}
					<div class="playtime">
						<Icon name="instance:clock" size={12} />
						<span
							>{t("instanceView.playtime", {
								time: playtimeLabel,
							})}</span
						>
					</div>
				{/if}
			</div>
		</div>
		<div
			class="compact-content"
			class:visible={activeSection !== "detalles"}
		>
			<button
				type="button"
				class="back-btn"
				aria-label={t("instanceView.tabs.details")}
				onclick={() => (activeSection = "detalles")}
			>
				<Icon name="ui:chevron-left" size={18} />
			</button>
			{#if isDefaultIcon}
				<div
					class="compact-icon compact-icon--accent"
					role="img"
					aria-label={instance.name}
				></div>
			{:else}
				<img
					class="compact-icon"
					src={getDisplayIconSrc(instance.icon)}
					alt={instance.name}
				/>
			{/if}
			<div class="compact-title-area">
				<span class="compact-title">{instance.name}</span>
				<span class="compact-version">{instance.version}</span>
			</div>
			<div class="compact-spacer"></div>
			<div class="launch-area">
				{@render launchButton()}
			</div>
		</div>
	</div>
</header>

<style>
	.instance-header {
		position: relative;
		flex-shrink: 0;
		z-index: 10;
		height: 180px;
		transition: height 0.25s ease;
	}
	.instance-header.compact {
		height: 56px;
	}

	.header-body {
		position: relative;
		height: 100%;
		overflow: hidden;
	}

	.header-bg {
		position: absolute;
		inset: 0;
		bottom: -40px;
		z-index: -1;
		pointer-events: none;
		background: var(--bg-sidebar);
		-webkit-backdrop-filter: blur(12px);
		backdrop-filter: blur(12px);
		-webkit-mask-image: linear-gradient(
			black 0%,
			black 30%,
			transparent 80%
		);
		mask-image: linear-gradient(black 0%, black 30%, transparent 80%);
		border-bottom: 1px solid var(--border);
	}

	.header-content {
		position: absolute;
		inset: 0;
		padding: 24px 24px 16px;
		display: flex;
		flex-direction: column;
		gap: 10px;
		transition:
			transform 0.22s ease,
			opacity 0.18s ease;
		opacity: 1;
		transform: translateX(0);
	}
	.header-content:not(.visible) {
		opacity: 0;
		transform: translateX(-100%);
		pointer-events: none;
	}

	.title-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 16px;
	}

	.title-left {
		display: flex;
		align-items: center;
		gap: 12px;
		min-width: 0;
	}

	.instance-icon {
		width: 36px;
		height: 36px;
		border-radius: 6px;
		object-fit: contain;
		flex-shrink: 0;
	}

	.instance-title {
		font-size: 2rem;
		font-weight: 800;
		color: var(--text-primary);
		margin: 0;
		letter-spacing: -0.5px;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.extension-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 16px;
	}

	.extension-left {
		display: flex;
		align-items: center;
		gap: 6px;
		min-width: 0;
		overflow: hidden;
	}

	.meta-chip {
		display: inline-flex;
		align-items: center;
		gap: 5px;
		font-size: 0.7rem;
		font-weight: 600;
		padding: 3px 8px;
		border-radius: 5px;
		background: var(--bg-card);
		color: var(--text-secondary);
		border: 1px solid var(--border);
		white-space: nowrap;
	}

	.loader-icon {
		width: 14px;
		height: 14px;
		object-fit: contain;
		flex-shrink: 0;
		border-radius: 2px;
	}

	.meta-sep {
		color: var(--text-tertiary);
		font-size: 0.7rem;
	}

	.inline-status {
		display: flex;
		align-items: center;
		gap: 5px;
		min-width: 0;
		overflow: hidden;
	}

	.inline-status.status-idle {
		color: var(--text-tertiary);
	}

	.inline-status.status-starting {
		color: var(--color-status-starting);
	}

	.inline-status.status-started {
		color: var(--color-status-started);
	}

	.inline-status.status-error {
		color: var(--color-error);
	}

	.inline-status .status-text {
		font-size: 0.7rem;
		font-weight: 600;
		white-space: nowrap;
		flex-shrink: 0;
	}

	.log-snippet {
		font-size: 0.65rem;
		font-weight: 500;
		color: inherit;
		opacity: 0;
		max-width: 0;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		transition:
			max-width 0.25s ease,
			opacity 0.25s ease,
			margin-left 0.25s ease;
		pointer-events: none;
	}

	.inline-status:hover .log-snippet {
		opacity: 0.7;
		max-width: 360px;
		margin-left: 2px;
	}

	.status-spin {
		animation: spin 1.2s linear infinite;
	}

	@keyframes spin {
		from {
			transform: rotate(0deg);
		}
		to {
			transform: rotate(-360deg);
		}
	}

	.launch-area {
		flex-shrink: 0;
	}

	.launch-btn {
		box-sizing: border-box;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: 8px;
		background: var(--accent);
		color: var(--accent-text);
		border: 1px solid transparent;
		min-width: 145px;
		padding: 10px 28px;
		border-radius: var(--border-radius-sm);
		font-size: 0.8rem;
		font-weight: 800;
		cursor: pointer;
		text-transform: uppercase;
		letter-spacing: 0.5px;
		box-shadow: var(--play-button-shadow, var(--shadow-md));
		transition:
			background 0.2s ease,
			box-shadow 0.2s ease;
	}

	.launch-btn:hover:not(:disabled) {
		background: var(--accent-hover);
		box-shadow: var(--play-button-shadow-hover, var(--shadow-lg));
	}

	.launch-btn:active:not(:disabled) {
		transform: scale(0.98);
	}

	.launch-btn:disabled {
		background: var(--surface-hover);
		color: var(--text-muted);
		cursor: not-allowed;
		box-shadow: none;
		border: 1px solid var(--border);
	}

	.launch-btn.downloading {
		position: relative;
		padding-inline: 16px;
		color: var(--text-secondary);
	}

	.launch-label {
		display: inline-flex;
		align-items: center;
		gap: 8px;
		width: max-content;
		flex-shrink: 0;
		white-space: nowrap;
	}

	.download-border {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
		overflow: visible;
		pointer-events: none;
	}

	.download-border rect {
		fill: none;
		stroke: var(--accent);
		stroke-width: 2px;
		rx: var(--border-radius-sm);
	}

	.download-percent {
		font-variant-numeric: tabular-nums;
		color: var(--accent);
	}

	.details-row {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 12px;
	}

	.path-row {
		display: flex;
		align-items: center;
		gap: 5px;
		min-width: 0;
		color: var(--text-tertiary);
		font-size: 0.65rem;
	}

	.path-text {
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		max-width: 300px;
	}

	.icon-btn {
		background: transparent;
		border: none;
		color: var(--text-tertiary);
		cursor: pointer;
		padding: 2px;
		display: flex;
		align-items: center;
		transition: color 0.15s;
		flex-shrink: 0;
	}

	.icon-btn:hover {
		color: var(--text-primary);
	}

	.playtime {
		display: flex;
		align-items: center;
		gap: 5px;
		font-size: 0.65rem;
		color: var(--text-tertiary);
		white-space: nowrap;
	}

	.playtime :global(.icon-svg) {
		flex-shrink: 0;
	}

	.actions-row {
		display: flex;
		gap: 4px;
	}

	.action-btn {
		height: 30px;
		padding: 0 6px;
		border-radius: 7px;
		background: transparent;
		border: 1px solid var(--border);
		color: var(--text-tertiary);
		cursor: pointer;
		display: flex;
		align-items: center;
		justify-content: center;
		transition: all 0.15s ease;
	}

	.action-btn :global(.icon-svg) {
		flex-shrink: 0;
	}

	.action-btn:hover {
		background: var(--bg-card);
		color: var(--text-primary);
		border-color: var(--text-tertiary);
	}

	.action-label {
		font-size: 0.68rem;
		font-weight: 500;
		white-space: nowrap;
		min-width: 0;
		max-width: 0;
		margin-left: 0;
		opacity: 0;
		overflow: hidden;
		transition:
			max-width 0.4s cubic-bezier(0.4, 0, 0.2, 1),
			opacity 0.4s cubic-bezier(0.4, 0, 0.2, 1);
		pointer-events: none;
	}

	.action-btn:hover .action-label {
		max-width: 120px;
		margin-left: 4px;
		opacity: 1;
	}

	.compact-content {
		position: absolute;
		inset: 0;
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 12px 24px;
		transition:
			transform 0.22s ease,
			opacity 0.18s ease;
		opacity: 1;
		transform: translateX(0);
	}
	.compact-content:not(.visible) {
		opacity: 0;
		transform: translateX(100%);
		pointer-events: none;
	}

	.back-btn {
		display: flex;
		align-items: center;
		justify-content: center;
		background: transparent;
		border: none;
		color: var(--text-tertiary);
		cursor: pointer;
		padding: 4px;
		border-radius: 6px;
		transition: color 0.15s;
		flex-shrink: 0;
	}

	.back-btn:hover {
		color: var(--text-primary);
	}

	.compact-icon {
		width: 28px;
		height: 28px;
		border-radius: 5px;
		object-fit: contain;
		flex-shrink: 0;
	}

	.instance-icon--accent {
		background-color: var(--text-primary);
		-webkit-mask: url("/images/cubic.svg") center/contain no-repeat;
		mask: url("/images/cubic.svg") center/contain no-repeat;
	}

	.compact-icon--accent {
		background-color: var(--text-primary);
		-webkit-mask: url("/images/cubic.svg") center/contain no-repeat;
		mask: url("/images/cubic.svg") center/contain no-repeat;
	}

	.compact-title-area {
		display: flex;
		align-items: center;
		gap: 8px;
		min-width: 0;
	}

	.compact-title {
		font-size: 1rem;
		font-weight: 700;
		color: var(--text-primary);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.compact-version {
		font-size: 0.68rem;
		font-weight: 600;
		padding: 2px 7px;
		border-radius: 4px;
		background: var(--bg-card);
		color: var(--text-secondary);
		border: 1px solid var(--border);
		white-space: nowrap;
		flex-shrink: 0;
	}

	.compact-spacer {
		flex: 1;
		min-width: 8px;
	}

	@media (max-width: 550px) {
		.path-text {
			max-width: 160px;
		}
	}
</style>
