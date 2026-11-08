<script lang="ts">
	import type { CurseForgeModpackInfo, MrpackInfo } from "$lib/types/types";
	import { t } from "$lib/i18n";

	type PackInfoData = MrpackInfo | CurseForgeModpackInfo;

	let {
		packInfo,
		format = "modrinth",
		onChangeFile,
	}: {
		packInfo: PackInfoData;
		format?: "modrinth" | "curseforge";
		onChangeFile: () => void;
	} = $props();

	function getPackName(info: PackInfoData): string {
		return info.name;
	}
	function getPackVersion(info: PackInfoData): string {
		return "version_id" in info ? info.version_id : info.version;
	}
	function getPackAuthor(info: PackInfoData): string | null {
		return "author" in info ? (info.author ?? null) : null;
	}
	function getPackMcVersion(info: PackInfoData): string | null {
		return info.minecraft_version;
	}
	function getPackLoader(info: PackInfoData): string | null {
		return info.loader;
	}
	function getPackLoaderVersion(info: PackInfoData): string | null {
		return info.loader_version;
	}
	function getPackSummary(info: PackInfoData): string | null {
		return info.summary ?? null;
	}
	function getPackFileCount(info: PackInfoData): number {
		return info.file_count;
	}
</script>

<div class="pack-info">
	<div class="pack-header">
		{#if packInfo.icon}
			<img src={packInfo.icon} alt="" class="pack-icon" />
		{/if}
		<div class="pack-header-info">
			<div class="info-row">
				<span class="info-label">Pack</span>
				<span class="info-value">{getPackName(packInfo)}</span>
			</div>
			<div class="info-row">
				<span class="info-label">Versión</span>
				<span class="info-value">{getPackVersion(packInfo)}</span>
			</div>
			{#if getPackAuthor(packInfo)}
				<div class="info-row">
					<span class="info-label">{t("mrpackExport.author")}</span>
					<span class="info-value">{getPackAuthor(packInfo)}</span>
				</div>
			{/if}
			{#if getPackSummary(packInfo)}
				<div class="info-row">
					<span class="info-label">Descripción</span>
					<span class="info-value summary"
						>{getPackSummary(packInfo)}</span
					>
				</div>
			{/if}
			<div class="info-row">
				<span class="info-label">Minecraft</span>
				<span class="info-value"
					>{getPackMcVersion(packInfo) ?? "—"}</span
				>
			</div>
			<div class="info-row">
				<span class="info-label">Loader</span>
				<span class="info-value"
					>{getPackLoader(packInfo) ??
						"Vanilla"}{getPackLoaderVersion(packInfo)
						? " " + getPackLoaderVersion(packInfo)
						: ""}</span
				>
			</div>
			<div class="info-row">
				<span class="info-label">Formato</span>
				<span class="info-value"
					>{format === "curseforge" ? "CurseForge" : "Modrinth"}</span
				>
			</div>
			<div class="info-row">
				<span class="info-label">Archivos</span>
				<span class="info-value"
					>{getPackFileCount(packInfo)} mods/archivos</span
				>
			</div>
		</div>
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

	.pack-header {
		display: flex;
		gap: 12px;
		align-items: flex-start;
	}

	.pack-icon {
		width: 56px;
		height: 56px;
		border-radius: var(--border-radius-sm);
		object-fit: cover;
		flex-shrink: 0;
		background: var(--bg-card);
	}

	.pack-header-info {
		flex: 1;
		display: flex;
		flex-direction: column;
		gap: 4px;
		min-width: 0;
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
		min-width: 0;
		overflow-wrap: anywhere;
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
