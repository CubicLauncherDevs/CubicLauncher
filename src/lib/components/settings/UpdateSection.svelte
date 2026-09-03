<script lang="ts">
	import { t } from "$lib/i18n";
	import CheckIcon from "$lib/icons/CheckIcon.svelte";
	import DownloadIcon from "$lib/icons/DownloadIcon.svelte";
	import Icon from "$lib/icons/Icon.svelte";

	let {
		currentVersion,
		pendingUpdate,
		updateProgress,
		updateDownloaded,
		checking,
		downloading,
		installing,
		onCheck,
		onDownload,
		onInstall,
	}: {
		currentVersion: string;
		pendingUpdate: { version?: string; body?: string } | null;
		updateProgress: number;
		updateDownloaded: boolean;
		checking: boolean;
		downloading: boolean;
		installing: boolean;
		onCheck: () => void;
		onDownload: () => void;
		onInstall: () => void;
	} = $props();
</script>

<div class="card">
	<div class="main-row">
		<div class="content">
			<div class="version-row">
				<div class="ver-info">
					<span class="ver-label"
						>{t("settings.launcher.currentVersion")}</span
					>
					<span class="ver-value">v{currentVersion}</span>
				</div>
				{#if pendingUpdate}
					<div class="ver-info">
						<span class="ver-label"
							>{t("settings.launcher.available")}</span
						>
						<span class="ver-value ver-available"
							>v{pendingUpdate.version}</span
						>
					</div>
				{:else}
					<div class="ver-info">
						<span class="ver-label"
							>{t("settings.launcher.status")}</span
						>
						<span class="ver-value ver-ok"
							>{t("settings.launcher.updateOk")}</span
						>
					</div>
				{/if}
			</div>
		</div>
		<div class="action">
			{#if checking}
				<button
					type="button"
					class="action-btn"
					disabled
					aria-label={t("settings.launcher.checkingBtn")}
				>
					<span class="spinner"></span>
				</button>
			{:else if downloading}
				<button
					type="button"
					class="action-btn"
					disabled
					aria-label={t("settings.launcher.downloadingBtn")}
				>
					<span class="spinner"></span>
				</button>
			{:else if installing}
				<button
					type="button"
					class="action-btn"
					disabled
					aria-label={t("settings.launcher.installingBtn")}
				>
					<span class="spinner"></span>
				</button>
			{:else if pendingUpdate && !updateDownloaded}
				<button
					type="button"
					class="action-btn"
					onclick={onDownload}
					title={t("settings.launcher.downloadInstallBtn")}
				>
					<DownloadIcon size={20} />
				</button>
			{:else if pendingUpdate && updateDownloaded}
				<button
					type="button"
					class="action-btn"
					onclick={onInstall}
					title={t("settings.launcher.installUpdateBtn")}
				>
					<CheckIcon size={16} />
				</button>
			{:else}
				<button
					type="button"
					class="action-btn"
					onclick={onCheck}
					title={t("settings.launcher.searchBtn")}
				>
					<Icon name="instance:clock" size={18} />
				</button>
			{/if}
		</div>
	</div>

	{#if pendingUpdate?.body}
		<div class="bottom-section">
			<div class="notes">
				<span class="notes-label">{t("settings.launcher.notes")}</span>
				<p class="notes-text">{pendingUpdate.body}</p>
			</div>
		</div>
	{/if}

	{#if updateProgress > 0 && updateProgress < 100}
		<div class="bottom-section">
			<div class="progress-wrap">
				<div class="progress-track">
					<div
						class="progress-fill"
						style="width: {updateProgress}%"
					></div>
				</div>
				<span class="progress-pct">{updateProgress}%</span>
			</div>
		</div>
	{/if}
</div>

<style>
	.card {
		background: var(--bg-card);
		border: 1px solid var(--border-color);
		border-radius: var(--border-radius-sm);
		box-shadow:
			var(--shadow-sm),
			inset 0 1px 0 var(--surface-selected);
		overflow: hidden;
	}

	.main-row {
		display: flex;
	}

	.content {
		flex: 1;
		padding: 10px 14px;
		display: flex;
		flex-direction: column;
		justify-content: center;
		min-width: 0;
	}

	.version-row {
		display: flex;
		gap: 20px;
	}

	.ver-info {
		display: flex;
		flex-direction: column;
		gap: 1px;
	}

	.ver-label {
		font-size: 0.6rem;
		text-transform: uppercase;
		letter-spacing: 0.8px;
		color: var(--text-muted);
		font-weight: 600;
	}

	.ver-value {
		font-size: 0.85rem;
		font-weight: 700;
		color: var(--text-primary);
	}

	.ver-available {
		color: var(--color-warning);
	}

	.ver-ok {
		color: var(--color-success);
	}

	.action {
		border-left: 1px solid var(--border-color);
		display: flex;
		align-items: center;
		justify-content: center;
		flex-shrink: 0;
	}

	.action-btn {
		background: transparent;
		border: none;
		color: var(--text-secondary);
		width: 48px;
		height: 100%;
		cursor: pointer;
		display: flex;
		align-items: center;
		justify-content: center;
		transition:
			color 0.15s,
			background 0.15s;
	}

	.action-btn:hover:not(:disabled) {
		color: var(--text-primary);
		background: var(--surface-selected);
	}

	.action-btn:disabled {
		opacity: 0.4;
		cursor: not-allowed;
	}

	.spinner {
		width: 14px;
		height: 14px;
		border: 2px solid var(--border-color);
		border-top-color: var(--text-secondary);
		border-radius: 50%;
		animation: spin 0.7s linear infinite;
		will-change: transform;
		display: inline-block;
	}

	@keyframes spin {
		to {
			transform: rotate(360deg);
		}
	}

	.bottom-section {
		border-top: 1px solid var(--border-color);
	}

	.notes {
		padding: 10px 14px;
	}

	.notes-label {
		display: block;
		font-size: 0.6rem;
		text-transform: uppercase;
		letter-spacing: 0.8px;
		color: var(--text-muted);
		font-weight: 600;
		margin-bottom: 4px;
	}

	.notes-text {
		font-size: 0.75rem;
		color: var(--text-secondary);
		line-height: 1.5;
		white-space: pre-wrap;
	}

	.progress-wrap {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 8px 14px;
	}

	.progress-track {
		flex: 1;
		height: 4px;
		background: var(--bg-input);
		border-radius: 0;
		overflow: hidden;
	}

	.progress-fill {
		height: 100%;
		background: var(--accent);
		transition: width 0.3s ease;
	}

	.progress-pct {
		font-size: 0.7rem;
		font-weight: 700;
		color: var(--text-secondary);
		min-width: 30px;
		text-align: right;
	}
</style>
