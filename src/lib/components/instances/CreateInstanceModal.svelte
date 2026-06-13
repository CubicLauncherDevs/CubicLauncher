<script lang="ts">
import {
	createInstance,
	getInstalledVersions,
	parseMrpack,
	installMrpack,
	parseCurseManifest,
	parseCurseManifestAndInstall,
} from "$lib/api/cubicApi";
import { INSTANCE_LOGOS } from "$lib/icons/logos";
import Select from "$lib/components/layout/Select.svelte";
import ModalBase from "$lib/components/layout/ModalBase.svelte";
import FtbModpackBrowser from "$lib/components/instances/FtbModpackBrowser.svelte";
import { t } from "$lib/i18n";
import type { MrpackInfo, CurseManifestInfo, InstallResultInfo } from "$lib/types/types";
import { open as openDialog } from "@tauri-apps/plugin-dialog";

	let {
		open = $bindable(),
		mrpackPath = $bindable<string | null>(null),
		oncreated,
	} = $props<{
		open: boolean;
		mrpackPath?: string | null;
		oncreated?: () => void;
	}>();

	// Manual mode state
	let name = $state("");
	let selectedVersion = $state("");
	let selectedIcon = $state<string | null>(null);
	let versions = $state<string[]>([]);
	let versionOptions = $derived(
		versions.map((v) => ({ value: v, label: v })),
	);

	// Import mode state
	type AnyPackInfo = MrpackInfo | CurseManifestInfo;
	let packInfo = $state<AnyPackInfo | null>(null);
	let packType = $state<"mrpack" | "curse">("mrpack");
	let parsing = $state(false);

	let loading = $state(false);
	let error = $state<string | null>(null);

	type Mode = "manual" | "import";
	let mode = $state<Mode>("manual");

	let showFtbBrowser = $state(false);

	$effect(() => {
		if (open) {
			fetchVersions();
		}
	});

	$effect(() => {
		if (open && mrpackPath) {
			mode = "import";
			loadPackInfo();
		}
	});

	async function fetchVersions() {
		const rawVersions = await getInstalledVersions();
		versions = rawVersions.sort((a, b) =>
			b.localeCompare(a, undefined, {
				numeric: true,
				sensitivity: "base",
			}),
		);

		if (versions.length > 0 && !selectedVersion) {
			selectedVersion = versions[0];
		}
	}

	async function loadPackInfo() {
		if (!mrpackPath) return;
		parsing = true;
		error = null;
		try {
			const isZip = mrpackPath.toLowerCase().endsWith(".zip");

			if (isZip) {
				const info = await parseCurseManifest(mrpackPath);
				if (info) {
					packInfo = info;
					packType = "curse";
					name = info.name;
					const loaderIcon = selectIconForLoader(info.loader);
					if (loaderIcon) selectedIcon = loaderIcon;
				} else {
					error = "No se pudo leer el archivo. Asegúrate de que sea un modpack de CurseForge/FTB válido.";
				}
			} else {
				const info = await parseMrpack(mrpackPath);
				if (info) {
					packInfo = info;
					packType = "mrpack";
					name = info.name;
					const loaderIcon = selectIconForLoader(info.loader);
					if (loaderIcon) selectedIcon = loaderIcon;
				} else {
					error = "No se pudo leer el archivo .mrpack";
				}
			}
		} finally {
			parsing = false;
		}
	}

	function selectIconForLoader(loader: string | null): string | null {
		if (!loader) return null;
		const l = loader.toLowerCase();
		if (l === "fabric") return "/images/instances/fabric.png";
		if (l === "forge") return "/images/instances/forge.png";
		if (l === "neoforge" || l === "neo") return "/images/instances/modth.png";
		if (l === "quilt") return "/images/instances/vanilla.png";
		return null;
	}

	function getPackName(info: AnyPackInfo): string {
		return info.name;
	}

	function getPackVersion(info: AnyPackInfo): string {
		return "version_id" in info ? info.version_id : info.version;
	}

	function getPackMcVersion(info: AnyPackInfo): string | null {
		return "minecraft_version" in info ? info.minecraft_version : null;
	}

	function getPackLoader(info: AnyPackInfo): string | null {
		return "loader" in info ? info.loader : null;
	}

	function getPackLoaderVersion(info: AnyPackInfo): string | null {
		return "loader_version" in info ? info.loader_version : null;
	}

	function getPackSummary(info: AnyPackInfo): string | null {
		return "summary" in info ? (info.summary ?? null) : null;
	}

	function getPackFileCount(info: AnyPackInfo): number {
		return "file_count" in info ? info.file_count : 0;
	}

	async function handleCreate() {
		if (mode === "import" && mrpackPath) {
			await handleImport();
		} else {
			await handleManualCreate();
		}
	}

	async function handleManualCreate() {
		if (!name.trim()) {
			error = t("createInstance.emptyNameErr");
			return;
		}

		if (!selectedVersion) {
			error = t("createInstance.noVersionsErr");
			return;
		}

		loading = true;
		error = null;

		try {
			await createInstance(
				name,
				selectedVersion,
				selectedIcon,
				() => {
					open = false;
					resetState();
					oncreated?.();
				},
				(err: unknown) => {
					error = t("createInstance.createErr");
					console.error(err);
				},
			);
		} finally {
			loading = false;
		}
	}

	async function handleImport() {
		if (!mrpackPath || !name.trim()) return;

		loading = true;
		error = null;

		try {
			if (packType === "curse") {
				const result = await parseCurseManifestAndInstall(
					mrpackPath,
					name.trim(),
					() => {
						open = false;
						mrpackPath = null;
						resetState();
						oncreated?.();
					},
					(err: unknown) => {
						error = `Error al importar: ${err}`;
					},
				);
				if (!result) {
					error = "Error al importar el modpack";
				}
			} else {
				const result = await installMrpack(
					mrpackPath,
					name.trim(),
					() => {
						open = false;
						mrpackPath = null;
						resetState();
						oncreated?.();
					},
					(err: unknown) => {
						error = `Error al importar: ${err}`;
					},
				);
				if (!result) {
					error = "Error al importar el modpack";
				}
			}
		} finally {
			loading = false;
		}
	}

	async function selectMrpackFile() {
		try {
			const selected = await openDialog({
				multiple: false,
				filters: [{ name: "Modpacks", extensions: ["mrpack", "zip"] }],
			});
			if (selected) {
				mrpackPath = selected;
				loadPackInfo();
			}
		} catch (e) {
			console.error("Error selecting file:", e);
		}
	}

	function resetState() {
		name = "";
		selectedVersion = "";
		selectedIcon = null;
		versions = [];
		error = null;
		parsing = false;
		packInfo = null;
		packType = "mrpack";
		loading = false;
		mode = "manual";
		showFtbBrowser = false;
	}

	function reset() {
		open = false;
		mrpackPath = null;
		resetState();
	}

	function switchMode(newMode: Mode) {
		mode = newMode;
		error = null;
		if (newMode === "import" && mrpackPath) {
			loadPackInfo();
		}
	}

	function onFtbInstalled() {
		open = false;
		resetState();
		oncreated?.();
	}
</script>

<ModalBase bind:open title={t("createInstance.title")} onclose={reset}>
	<div class="tab-bar">
		<button
			type="button"
			class="tab-btn"
			class:active={mode === "manual"}
			onclick={() => switchMode("manual")}
		>
			Manual
		</button>
		<button
			type="button"
			class="tab-btn"
			class:active={mode === "import"}
			onclick={() => switchMode("import")}
		>
			Desde Modpack
		</button>
	</div>

	{#if mode === "manual"}
		<div>
			<div class="create-layout">
				<div class="create-logo-section">
					<span class="input-label">{t("createInstance.iconLabel")}</span>
					<div class="create-logo-preview">
						{#if selectedIcon}
							<img src={selectedIcon} alt="Logo" />
							<button
								type="button"
								class="create-logo-clear"
								onclick={() => (selectedIcon = null)}
								onmouseenter={(e) => (e.currentTarget.style.opacity = "1")}
								onmouseleave={(e) => (e.currentTarget.style.opacity = "0")}
							>Limpiar</button>
						{:else}
							<svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="color: var(--text-secondary); opacity: 0.5;">
								<rect x="3" y="3" width="18" height="18" rx="2" ry="2"></rect>
								<circle cx="8.5" cy="8.5" r="1.5"></circle>
								<polyline points="21 15 16 10 5 21"></polyline>
							</svg>
						{/if}
					</div>
				</div>

				<div class="create-details-section">
					<div class="input-group">
						<span class="input-label">{t("createInstance.nameLabel")}</span>
						<input
							type="text"
							class="text-input"
							bind:value={name}
							placeholder={t("createInstance.namePlaceholder")}
							disabled={loading}
							onkeydown={(e) => e.key === "Enter" && handleCreate()}
						/>
					</div>

					<div class="input-group">
						<Select
							label={t("createInstance.versionLabel")}
							bind:value={selectedVersion}
							options={versionOptions}
							disabled={loading || versions.length === 0}
							placeholder={t("createInstance.noVersionsErr")}
						/>
					</div>
				</div>
			</div>

			<div class="create-divider"></div>

			<div class="input-group">
				<span class="input-label">Seleccionar Icono</span>
				<div class="create-icon-grid">
					{#each INSTANCE_LOGOS as iconName (iconName)}
						{@const iconPath = `/images/instances/${iconName}`}
						<button
							type="button"
							class="icon-option"
							class:selected={selectedIcon === iconPath}
							onclick={() => (selectedIcon = selectedIcon === iconPath ? null : iconPath)}
							title={iconName}
						>
							<img src={iconPath} alt={iconName} />
						</button>
					{/each}
				</div>
			</div>
		</div>
	{/if}

	{#if mode === "import"}
		<div>
			{#if parsing}
				<div class="parsing-state">
					<p>Analizando modpack...</p>
				</div>
			{:else if !mrpackPath}
				<div class="drop-zone">
					<p>Arrastra un archivo .mrpack o .zip aquí</p>
					<span class="drop-or">o</span>
					<button type="button" class="btn-secondary" onclick={selectMrpackFile}>
						Seleccionar archivo
					</button>
				</div>

				<div class="ftb-section">
					<div class="ftb-divider"><span>o descarga desde</span></div>
					<button type="button" class="ftb-open-btn" onclick={() => (showFtbBrowser = !showFtbBrowser)}>
						<img src="/images/instances/ftb.png" alt="FTB" class="ftb-icon" />
						FTB Modpacks
					</button>

					{#if showFtbBrowser}
						<FtbModpackBrowser onInstalled={onFtbInstalled} />
					{/if}
				</div>
			{:else if packInfo}
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
						<span class="info-value">{getPackLoader(packInfo) ?? "Vanilla"}{getPackLoaderVersion(packInfo) ? " " + getPackLoaderVersion(packInfo) : ""}</span>
					</div>
					<div class="info-row">
						<span class="info-label">Formato</span>
						<span class="info-value">{packType === "mrpack" ? "Modrinth" : "CurseForge/FTB"}</span>
					</div>
					<div class="info-row">
						<span class="info-label">Archivos</span>
						<span class="info-value">{getPackFileCount(packInfo)} mods/archivos</span>
					</div>
				</div>

				<div class="input-group">
					<span class="input-label">Nombre de la instancia</span>
					<input
						type="text"
						class="text-input"
						bind:value={name}
						placeholder="Nombre de la instancia"
						disabled={loading}
						onkeydown={(e) => e.key === "Enter" && handleCreate()}
					/>
				</div>
			{:else if error}
				<div class="error-msg">{error}</div>
				<div class="drop-zone">
					<p>Arrastra otro archivo .mrpack o .zip</p>
					<span class="drop-or">o</span>
					<button type="button" class="btn-secondary" onclick={selectMrpackFile}>
						Seleccionar archivo
					</button>
				</div>
			{/if}
		</div>
	{/if}

	{#if error && mode === "manual"}
		<div class="create-error">{error}</div>
	{/if}

	{#snippet footer()}
		<button
			type="button"
			class="btn-secondary"
			onclick={reset}
			disabled={loading}
		>
			{t("createInstance.cancel")}
		</button>
		<button
			type="button"
			class="btn-primary"
			onclick={handleCreate}
			disabled={loading || (mode === "import" && (!mrpackPath || !name.trim()))}
		>
			{loading
				? (mode === "import" ? "Importando..." : t("createInstance.creatingBtn"))
				: (mode === "import" ? "Importar Modpack" : t("createInstance.createBtn"))}
		</button>
	{/snippet}
</ModalBase>

<style>
	.tab-bar {
		display: flex;
		gap: 0;
		margin-bottom: 16px;
		border-bottom: 1px solid var(--border);
	}

	.tab-btn {
		flex: 1;
		padding: 10px 16px;
		background: none;
		border: none;
		border-bottom: 2px solid transparent;
		color: var(--text-secondary);
		font-size: 0.85rem;
		font-weight: 600;
		cursor: pointer;
		transition: color 0.15s, border-color 0.15s;
	}

	.tab-btn:hover {
		color: var(--text-primary);
	}

	.tab-btn.active {
		color: var(--text-primary);
		border-bottom-color: var(--accent);
	}

	.create-layout {
		display: flex;
		gap: 20px;
	}

	.create-logo-section {
		display: flex;
		flex-direction: column;
		gap: 12px;
		width: 100px;
		align-items: center;
		flex-shrink: 0;
	}

	.create-logo-preview {
		width: 80px;
		height: 80px;
		border-radius: 12px;
		background: rgba(255, 255, 255, 0.03);
		border: 2px dashed var(--border);
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 12px;
		position: relative;
		overflow: hidden;
	}

	.create-logo-preview img {
		width: 100%;
		height: 100%;
		object-fit: contain;
		filter: drop-shadow(0 4px 8px rgba(0, 0, 0, 0.3));
	}

	.create-logo-clear {
		position: absolute;
		inset: 0;
		background: rgba(0, 0, 0, 0.6);
		border: none;
		color: white;
		opacity: 0;
		cursor: pointer;
		transition: opacity 0.2s;
		font-size: 0.7rem;
		font-weight: bold;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.create-logo-clear:hover {
		opacity: 1;
	}

	.create-details-section {
		display: flex;
		flex-direction: column;
		gap: 16px;
		flex: 1;
	}

	.create-divider {
		height: 1px;
		background: var(--border);
		margin: 16px 0 12px;
	}

	.create-icon-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(44px, 1fr));
		gap: 8px;
		margin-top: 4px;
		max-height: 110px;
		overflow-y: auto;
		padding-right: 4px;
		padding-bottom: 4px;
	}

	.create-error {
		color: var(--color-error);
		font-size: 0.8rem;
		background: rgba(var(--color-error-rgb), 0.1);
		border: 1px solid rgba(var(--color-error-rgb), 0.2);
		border-radius: 6px;
		padding: 10px;
		text-align: center;
		font-weight: 500;
		margin-top: 8px;
	}

	.parsing-state,
	.drop-zone {
		padding: 32px 16px;
		text-align: center;
		color: var(--text-secondary);
		font-size: 0.9rem;
	}

	.drop-zone {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 12px;
		border: 2px dashed var(--border);
		border-radius: var(--border-radius-sm);
		margin: 8px 0;
	}

	.drop-or {
		font-size: 0.75rem;
		text-transform: uppercase;
		opacity: 0.5;
	}

	.pack-info {
		display: flex;
		flex-direction: column;
		gap: 8px;
		margin-bottom: 16px;
		padding: 12px;
		background: rgba(255, 255, 255, 0.02);
		border-radius: var(--border-radius-sm);
		border: 1px solid var(--border);
	}

	.info-row {
		display: flex;
		align-items: baseline;
		gap: 8px;
	}

	.info-label {
		font-size: 0.75rem;
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

	.input-group {
		margin-top: 12px;
	}

	.input-label {
		font-size: 0.75rem;
		font-weight: 600;
		color: var(--text-secondary);
		text-transform: uppercase;
		letter-spacing: 0.5px;
		margin-bottom: 6px;
		display: block;
	}

	.error-msg {
		color: var(--color-error);
		font-size: 0.8rem;
		background: rgba(var(--color-error-rgb), 0.1);
		border: 1px solid rgba(var(--color-error-rgb), 0.2);
		border-radius: 6px;
		padding: 10px;
		text-align: center;
		font-weight: 500;
		margin-top: 8px;
	}

	.ftb-section {
		margin-top: 16px;
	}

	.ftb-divider {
		display: flex;
		align-items: center;
		gap: 12px;
		margin-bottom: 12px;
		font-size: 0.75rem;
		color: var(--text-secondary);
		text-transform: uppercase;
		letter-spacing: 0.5px;
	}

	.ftb-divider::before,
	.ftb-divider::after {
		content: "";
		flex: 1;
		height: 1px;
		background: var(--border);
	}

	.ftb-open-btn {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 8px;
		width: 100%;
		padding: 10px;
		background: rgba(255, 255, 255, 0.03);
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		color: var(--text-primary);
		font-size: 0.85rem;
		font-weight: 600;
		cursor: pointer;
		transition: background 0.15s, border-color 0.15s;
	}

	.ftb-open-btn:hover {
		background: rgba(255, 255, 255, 0.06);
		border-color: var(--text-secondary);
	}

	.ftb-icon {
		width: 20px;
		height: 20px;
	}
</style>
