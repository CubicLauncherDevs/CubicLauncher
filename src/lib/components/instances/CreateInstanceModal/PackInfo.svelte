<script lang="ts">
	import type { MrpackInfo } from "$lib/types/types";

	let {
		packInfo,
		onChangeFile,
	}: {
		packInfo: MrpackInfo;
		onChangeFile: () => void;
	} = $props();

	function getPackName(info: MrpackInfo): string {
		return info.name;
	}
	function getPackVersion(info: MrpackInfo): string {
		return info.version_id;
	}
	function getPackMcVersion(info: MrpackInfo): string | null {
		return info.minecraft_version;
	}
	function getPackLoader(info: MrpackInfo): string | null {
		return info.loader;
	}
	function getPackLoaderVersion(info: MrpackInfo): string | null {
		return info.loader_version;
	}
	function getPackSummary(info: MrpackInfo): string | null {
		return info.summary ?? null;
	}
	function getPackFileCount(info: MrpackInfo): number {
		return info.file_count;
	}
</script>

<div class="pack-info">
	<div class="info-row">
		<span class="info-label">Pack</span>
		<span class="info-value">{getPackName(packInfo)}</span>
	</div>
	<div class="info-row">
		<span class="info-label">Versión</span>
		<span class="info-value">{getPackVersion(packInfo)}</span>
	</div>
	{#if getPackSummary(packInfo)}
		<div class="info-row">
			<span class="info-label">Descripción</span>
			<span class="info-value summary">{getPackSummary(packInfo)}</span>
		</div>
	{/if}
	<div class="info-row">
		<span class="info-label">Minecraft</span>
		<span class="info-value">{getPackMcVersion(packInfo) ?? "—"}</span>
	</div>
	<div class="info-row">
		<span class="info-label">Loader</span>
		<span class="info-value"
			>{getPackLoader(packInfo) ?? "Vanilla"}{getPackLoaderVersion(
				packInfo,
			)
				? " " + getPackLoaderVersion(packInfo)
				: ""}</span
		>
	</div>
	<div class="info-row">
		<span class="info-label">Formato</span>
		<span class="info-value">Modrinth</span>
	</div>
	<div class="info-row">
		<span class="info-label">Archivos</span>
		<span class="info-value">{getPackFileCount(packInfo)} mods/archivos</span>
	</div>
</div>
<button type="button" class="btn-change-file" onclick={onChangeFile}>
	Cambiar archivo
</button>

<style>
	.pack-info {
		display: flex;
		flex-direction: column;
		gap: 8px;
		padding: 12px;
		background: rgba(var(--accent-rgb, 255, 255, 255), 0.02);
		border-radius: var(--border-radius-sm);
		border: 1px solid var(--border);
	}

	.info-row {
		display: flex;
		align-items: baseline;
		gap: 8px;
	}

	.info-label {
		font-size: 0.7rem;
		font-weight: 600;
		color: var(--text-secondary);
		text-transform: uppercase;
		letter-spacing: 0.5px;
		min-width: 80px;
		flex-shrink: 0;
	}

	.info-value {
		font-size: 0.85rem;
		color: var(--text-primary);
	}

	.info-value.summary {
		font-size: 0.8rem;
		color: var(--text-secondary);
		line-height: 1.3;
	}

	.btn-change-file {
		align-self: flex-start;
		padding: 6px 12px;
		background: none;
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		color: var(--text-secondary);
		font-size: 0.75rem;
		cursor: pointer;
		transition:
			color 0.15s,
			border-color 0.15s;
	}

	.btn-change-file:hover {
		color: var(--text-primary);
		border-color: var(--text-secondary);
	}
</style>
