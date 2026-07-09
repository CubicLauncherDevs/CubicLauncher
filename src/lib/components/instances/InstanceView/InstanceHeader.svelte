<script lang="ts">
	import { t } from "$lib/i18n";
	import { invoke } from "@tauri-apps/api/core";
	import { listen } from "@tauri-apps/api/event";
	import type { InstanceDto } from "$lib/types/types";
	import { launcherStore } from "$lib/state/state.svelte";
	import { getLoaderLogo } from "$lib/icons/logos";

	let {
		instance,
		bannerState = "Idle",
		isDownloadingVersion = false,
		activeSection = $bindable("detalles"),
		onPlay = () => {},
	}: {
		instance: InstanceDto;
		bannerState: string;
		isDownloadingVersion: boolean;
		activeSection: string;
		onPlay: () => void;
	} = $props();

	const loaderIcon = $derived(getLoaderLogo(instance.loader));

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
	const lastPlayedLabel = $derived.by(() => {
		if (instance.last_played < 1) {
			return t("instanceView.neverPlayed");
		}
		return formatter.format(new Date(instance.last_played * 1000));
	});

	let lastLog = $state("");

	$effect(() => {
		const id = instance.uuid;
		lastLog = "";
		let destroyed = false;
		const unlistenPromise = listen<{
			id: string;
			lines: { line: string; stream: string; timestamp: number }[];
		}>("instance-log-batch", (event) => {
			if (
				!destroyed &&
				event.payload.id === id &&
				event.payload.lines.length > 0
			) {
				const last =
					event.payload.lines[event.payload.lines.length - 1];
				lastLog = last.line;
			}
		});
		return () => {
			destroyed = true;
			unlistenPromise.then((u) => u?.());
		};
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

<header class="instance-header" class:compact={activeSection !== "detalles"}>
	<div class="header-bg"></div>
	<div class="header-body">
	<div class="header-content" class:visible={activeSection === "detalles"}>
			<div class="title-row">
				<div class="title-left">
					<img
						class="instance-icon"
						src={instance.icon || "/images/cubic.svg"}
						alt={instance.name}
					/>
					<h1 class="instance-title">{instance.name}</h1>
				</div>
				<div class="actions-row">
					<button
						type="button"
						class="action-btn"
						onclick={() => openDir()}
					>
						<svg
							width="15"
							height="15"
							viewBox="0 0 24 24"
							fill="none"
							stroke="currentColor"
							stroke-width="2"
							stroke-linecap="round"
							stroke-linejoin="round"
						>
							<path
								d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"
							/>
						</svg>
						<span class="action-label"
							>{t("instanceView.options.folder")}</span
						>
					</button>
					<button
						type="button"
						class="action-btn"
						onclick={() => openDir("mods")}
					>
						<svg
							width="15"
							height="15"
							viewBox="0 0 24 24"
							fill="none"
							stroke="currentColor"
							stroke-width="2"
							stroke-linecap="round"
							stroke-linejoin="round"
						>
							<rect x="3" y="3" width="7" height="7" /><rect
								x="14"
								y="3"
								width="7"
								height="7"
							/><rect x="14" y="14" width="7" height="7" /><rect
								x="3"
								y="14"
								width="7"
								height="7"
							/>
						</svg>
						<span class="action-label"
							>{t("instanceView.tabs.mods")}</span
						>
					</button>
					<button
						type="button"
						class="action-btn"
						onclick={() => openDir("screenshots")}
					>
						<svg
							width="15"
							height="15"
							viewBox="0 0 24 24"
							fill="none"
							stroke="currentColor"
							stroke-width="2"
							stroke-linecap="round"
							stroke-linejoin="round"
						>
							<path
								d="M23 19a2 2 0 0 1-2 2H3a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h4l2-3h6l2 3h4a2 2 0 0 1 2 2z"
							/><circle cx="12" cy="13" r="4" />
						</svg>
						<span class="action-label"
							>{t("instanceView.tabs.screenshots")}</span
						>
					</button>
					<button
						type="button"
						class="action-btn"
						onclick={() => openDir("resourcepacks")}
					>
						<svg
							width="15"
							height="15"
							viewBox="0 0 24 24"
							fill="none"
							stroke="currentColor"
							stroke-width="2"
							stroke-linecap="round"
							stroke-linejoin="round"
						>
							<ellipse cx="12" cy="5" rx="9" ry="3" /><path
								d="M21 12c0 1.66-4 3-9 3s-9-1.34-9-3"
							/><path d="M3 5v14c0 1.66 4 3 9 3s9-1.34 9-3V5" />
						</svg>
						<span class="action-label"
							>{t("instanceView.tabs.resources")}</span
						>
					</button>
					<button
						type="button"
						class="action-btn"
						onclick={() => openLogs()}
					>
						<svg
							width="15"
							height="15"
							viewBox="0 0 24 24"
							fill="none"
							stroke="currentColor"
							stroke-width="2"
							stroke-linecap="round"
							stroke-linejoin="round"
						>
							<polyline points="16 18 22 12 16 6" /><polyline
								points="8 6 2 12 8 18"
							/>
						</svg>
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
							<svg
								width="12"
								height="12"
								viewBox="0 0 24 24"
								fill="none"
								stroke="currentColor"
								stroke-width="2.5"
								stroke-linecap="round"
								stroke-linejoin="round"
							>
								<polyline points="20 6 9 17 4 12" />
							</svg>
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
					{#if bannerState == "Started"}
						<button
							type="button"
							class="launch-btn"
							onclick={onPlay}
						>
							{t("instanceView.close")}
						</button>
					{:else if bannerState == "Starting"}
						<button type="button" class="launch-btn" disabled>
							{t("instanceView.playBtn")}
						</button>
					{:else if isDownloadingVersion}
						<button type="button" class="launch-btn" disabled>
							{t("instanceView.downloadingBtn")}
						</button>
					{:else}
						<button
							type="button"
							class="launch-btn"
							onclick={onPlay}
						>
							{t("instanceView.playBtn")}
						</button>
					{/if}
				</div>
			</div>

			<div class="details-row">
				<div class="path-row">
					<svg
						width="12"
						height="12"
						viewBox="0 0 24 24"
						fill="none"
						stroke="currentColor"
						stroke-width="2"
						stroke-linecap="round"
						stroke-linejoin="round"
					>
						<path
							d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"
						/>
					</svg>
					<span class="path-text" title={instance.path}
						>{instance.path}</span
					>
					<button
						type="button"
						class="icon-btn"
						onclick={() => openDir()}
						title={t("instanceView.details.location")}
					>
						<svg
							width="13"
							height="13"
							viewBox="0 0 24 24"
							fill="none"
							stroke="currentColor"
							stroke-width="2"
							stroke-linecap="round"
							stroke-linejoin="round"
						>
							<path
								d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"
							/><polyline points="15 3 21 3 21 9" /><line
								x1="10"
								y1="14"
								x2="21"
								y2="3"
							/>
						</svg>
					</button>
				</div>
				<div class="last-played">
					<svg
						width="12"
						height="12"
						viewBox="0 0 24 24"
						fill="none"
						stroke="currentColor"
						stroke-width="2"
						stroke-linecap="round"
						stroke-linejoin="round"
					>
						<circle cx="12" cy="12" r="10" /><polyline
							points="12 6 12 12 16 14"
						/>
					</svg>
					<span
						>{t("instanceView.lastPlayed").replace(
							"{date}",
							lastPlayedLabel,
						)}</span
					>
				</div>
			</div>
		</div>
	<div class="compact-content" class:visible={activeSection !== "detalles"}>
			<button
				type="button"
				class="back-btn"
				aria-label={t("instanceView.tabs.details")}
				onclick={() => (activeSection = "detalles")}
			>
				<svg
					width="18"
					height="18"
					viewBox="0 0 24 24"
					fill="none"
					stroke="currentColor"
					stroke-width="2"
					stroke-linecap="round"
					stroke-linejoin="round"
				>
					<line x1="19" y1="12" x2="5" y2="12" /><polyline
						points="12 19 5 12 12 5"
					/>
				</svg>
			</button>
			<img
				class="compact-icon"
				src={instance.icon || "/images/cubic.svg"}
				alt={instance.name}
			/>
			<div class="compact-title-area">
				<span class="compact-title">{instance.name}</span>
				<span class="compact-version">{instance.version}</span>
			</div>
			<div class="compact-spacer"></div>
			<div class="launch-area">
				{#if bannerState == "Started"}
					<button type="button" class="launch-btn" onclick={onPlay}>
						{t("instanceView.close")}
					</button>
				{:else if bannerState == "Starting"}
					<button type="button" class="launch-btn" disabled>
						{t("instanceView.playBtn")}
					</button>
				{:else if isDownloadingVersion}
					<button type="button" class="launch-btn" disabled>
						{t("instanceView.downloadingBtn")}
					</button>
				{:else}
					<button type="button" class="launch-btn" onclick={onPlay}>
						{t("instanceView.playBtn")}
					</button>
				{/if}
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
		transition: transform 0.22s ease, opacity 0.18s ease;
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
		color: #64b5f6;
	}

	.inline-status.status-started {
		color: #81c784;
	}

	.inline-status.status-error {
		color: #e57373;
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
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: 8px;
		background: white;
		color: black;
		border: none;
		min-width: 145px;
		padding: 10px 28px;
		border-radius: var(--border-radius-sm);
		font-size: 0.8rem;
		font-weight: 800;
		cursor: pointer;
		text-transform: uppercase;
		letter-spacing: 0.5px;
		box-shadow:
			inset 0 1px 3px rgba(0, 0, 0, 0.15),
			0 4px 15px rgba(0, 0, 0, 0.3);
		transition:
			background 0.2s ease,
			box-shadow 0.2s ease;
	}

	.launch-btn:hover:not(:disabled) {
		background: #f0f0f0;
		box-shadow: 0 6px 20px rgba(0, 0, 0, 0.4);
	}

	.launch-btn:active:not(:disabled) {
		transform: scale(0.98);
	}

	.launch-btn:disabled {
		background: rgba(255, 255, 255, 0.15);
		color: rgba(255, 255, 255, 0.35);
		cursor: not-allowed;
		box-shadow: none;
		border: 1px solid rgba(255, 255, 255, 0.1);
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

	.last-played {
		display: flex;
		align-items: center;
		gap: 5px;
		font-size: 0.65rem;
		color: var(--text-tertiary);
		white-space: nowrap;
	}

	.last-played svg {
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

	.action-btn svg {
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
		transition: transform 0.22s ease, opacity 0.18s ease;
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
