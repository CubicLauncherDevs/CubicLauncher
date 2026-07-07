<script lang="ts">
	import { t } from "$lib/i18n";
	import CheckIcon from "$lib/icons/CheckIcon.svelte";

	const dateFmt = new Intl.DateTimeFormat(undefined, { dateStyle: "medium" });

	let {
		version,
		filter,
		isInstalled,
		isDownloading,
		ondownload,
	}: {
		version: {
			id: string;
			version: string;
			game_version: string;
			type: string;
			stable: boolean;
			releaseTime: string;
		};
		filter: string;
		isInstalled: boolean;
		isDownloading: boolean;
		ondownload: () => void;
	} = $props();
</script>

<div
	class="version-item"
	style="display: flex; align-items: center; justify-content: space-between; padding: 12px; background: var(--bg-card); border: 1px solid var(--border-color); border-radius: 8px; height: 58px;"
>
	<div class="version-info">
		<div style="display: flex; align-items: center; gap: 8px;">
			<div style="font-weight: 600; font-size: 0.9rem;">
				{filter === "fabric" ? version.version : version.id}
			</div>
			{#if isInstalled}
				<span class="inst-badge"
					>{t("versionDownloader.installedTag")}</span
				>
			{/if}
		</div>
		<div
			style="font-size: 0.7rem; color: var(--text-muted); text-transform: uppercase; letter-spacing: 0.5px;"
		>
			{#if filter === "fabric"}
				Fabric Meta • {version.stable ? "STABLE" : "UNSTABLE"}
			{:else if filter === "quilt"}
				Quilt Meta • {version.stable ? "STABLE" : "UNSTABLE"}
			{:else if filter === "forge"}
				Forge • MC {version.game_version}
			{:else}
				{version.type} • {dateFmt.format(new Date(version.releaseTime))}
			{/if}
		</div>
	</div>

	{#if isInstalled}
		<div class="inst-icon">
			<CheckIcon size={10} />
		</div>
	{:else if isDownloading}
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
		<button type="button" class="download-btn" onclick={ondownload}>
			{t("versionDownloader.downloadBtn")}
		</button>
	{/if}
</div>

<style>
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
