<script lang="ts">
	import { t } from "$lib/i18n";
	import { invoke } from "@tauri-apps/api/core";
	import { listen } from "@tauri-apps/api/event";
	import type { InstanceDto } from "$lib/types/types";
	import { launcherStore } from "$lib/state/state.svelte";
	import { getLoaderLogo, getDisplayIconSrc } from "$lib/icons/logos";
	import Icon from "$lib/icons/Icon.svelte";

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
		<div
			class="header-content"
			class:visible={activeSection === "detalles"}
		>
			<div class="title-row">
				<div class="title-left">
					<img
						class="instance-icon"
						src={getDisplayIconSrc(instance.icon)}
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
						<Icon
							src="/images/icons/instance/folder.svg"
							size={15}
						/>
						<span class="action-label"
							>{t("instanceView.options.folder")}</span
						>
					</button>
					<button
						type="button"
						class="action-btn"
						onclick={() => openDir("mods")}
					>
						<Icon src="/images/icons/instance/grid.svg" size={15} />
						<span class="action-label"
							>{t("instanceView.tabs.mods")}</span
						>
					</button>
					<button
						type="button"
						class="action-btn"
						onclick={() => openDir("screenshots")}
					>
						<Icon
							src="/images/icons/instance/image.svg"
							size={15}
						/>
						<span class="action-label"
							>{t("instanceView.tabs.screenshots")}</span
						>
					</button>
					<button
						type="button"
						class="action-btn"
						onclick={() => openDir("resourcepacks")}
					>
						<Icon
							src="/images/icons/instance/database.svg"
							size={15}
						/>
						<span class="action-label"
							>{t("instanceView.tabs.resources")}</span
						>
					</button>
					<button
						type="button"
						class="action-btn"
						onclick={() => openLogs()}
					>
						<Icon src="/images/icons/instance/code.svg" size={15} />
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
							<Icon src="/images/icons/ui/check.svg" size={12} />
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
					<Icon src="/images/icons/instance/folder.svg" size={12} />
					<span class="path-text" title={instance.path}
						>{instance.path}</span
					>
					<button
						type="button"
						class="icon-btn"
						onclick={() => openDir()}
						title={t("instanceView.details.location")}
					>
						<Icon
							src="/images/icons/instance/external-link.svg"
							size={13}
						/>
					</button>
				</div>
				<div class="last-played">
					<Icon src="/images/icons/instance/clock.svg" size={12} />
					<span
						>{t("instanceView.lastPlayed", {
							date: lastPlayedLabel,
						})}</span
					>
				</div>
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
				<Icon src="/images/icons/ui/chevron-left.svg" size={18} />
			</button>
			<img
				class="compact-icon"
				src={getDisplayIconSrc(instance.icon)}
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
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: 8px;
		background: var(--accent);
		color: var(--accent-text);
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
		background: var(--accent-hover);
		box-shadow: 0 6px 20px rgba(0, 0, 0, 0.4);
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

	.last-played :global(.icon-svg) {
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
