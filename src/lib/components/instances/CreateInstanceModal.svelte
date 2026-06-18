<script lang="ts">
	import {
		createInstance,
		fetchAll,
		getInstalledVersions,
		parseMrpack,
		installMrpack,
	} from "$lib/api/cubicApi";
	import { INSTANCE_LOGOS } from "$lib/icons/logos";
	import Select from "$lib/components/layout/Select.svelte";
	import ModalBase from "$lib/components/layout/ModalBase.svelte";
	import { t } from "$lib/i18n";
	import type { MrpackInfo } from "$lib/types/types";
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

	// ── Step 1: Name + Icon ───────────────────────────────────────────────────
	let name = $state("");
	let selectedIcon = $state<string | null>(null);

	// ── Step 2: Version or Modpack ────────────────────────────────────────────
	type ContentSource = "version" | "modpack";
	let contentSource = $state<ContentSource>("version");

	// Version
	let selectedVersion = $state("");
	let versions = $state<string[]>([]);
	let versionOptions = $derived(
		versions.map((v) => ({ value: v, label: v })),
	);

	// Modpack
	let packInfo = $state<MrpackInfo | null>(null);
	let parsing = $state(false);

	// ── Common ────────────────────────────────────────────────────────────────
	let loading = $state(false);
	let error = $state<string | null>(null);
	let existingNames = $state<string[]>([]);

	// ── Steps ─────────────────────────────────────────────────────────────────
	let currentStep = $state(0);
	const TOTAL_STEPS = 2;
	let nameMsg = $state<string | null>(null);

	function validateName(): boolean {
		const trimmed = name.trim();
		if (!trimmed) {
			nameMsg = "createInstance.emptyNameErr";
			return false;
		}
		if (trimmed.length > 16) {
			nameMsg = "createInstance.nameTooLong";
			return false;
		}
		if (existingNames.includes(trimmed)) {
			nameMsg = "createInstance.nameExists";
			return false;
		}
		nameMsg = null;
		return true;
	}

	function nextStep() {
		if (currentStep === 0 && !validateName()) return;
		if (currentStep < TOTAL_STEPS - 1) currentStep++;
	}
	function prevStep() {
		if (currentStep > 0) currentStep--;
	}
	function isLastStep() {
		return currentStep === TOTAL_STEPS - 1;
	}

	// ── Effects ───────────────────────────────────────────────────────────────
	$effect(() => {
		if (open) {
			currentStep = 0;
			contentSource = "version";
			nameMsg = null;
			fetchVersions();
			fetchInstances();
		}
	});

	$effect(() => {
		if (open && mrpackPath) {
			contentSource = "modpack";
			loadPackInfo();
		}
	});

	// ── Fetch instances ───────────────────────────────────────────────────────
	async function fetchInstances() {
		const instances = await fetchAll();
		existingNames = instances.map((i) => i.name);
	}

	// ── Fetch versions ────────────────────────────────────────────────────────
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

	// ── Load pack info ────────────────────────────────────────────────────────
	async function loadPackInfo() {
		if (!mrpackPath) return;
		parsing = true;
		error = null;
		try {
			const info = await parseMrpack(mrpackPath);
			if (info) {
				packInfo = info;
				if (!name.trim()) name = info.name;
				const loaderIcon = selectIconForLoader(info.loader);
				if (loaderIcon) selectedIcon = loaderIcon;
			} else {
				error = "No se pudo leer el archivo .mrpack";
			}
		} finally {
			parsing = false;
		}
	}

	// ── Helpers ───────────────────────────────────────────────────────────────
	function selectIconForLoader(loader: string | null): string | null {
		if (!loader) return null;
		const l = loader.toLowerCase();
		if (l === "fabric") return "/images/instances/fabric.png";
		if (l === "forge") return "/images/instances/forge.png";
		if (l === "neoforge" || l === "neo")
			return "/images/instances/modth.png";
		if (l === "quilt") return "/images/instances/vanilla.png";
		return null;
	}

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

	// ── Create / Import ──────────────────────────────────────────────────────
	async function handleFinalAction() {
		if (contentSource === "modpack" && mrpackPath) {
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
			if (!result) error = "Error al importar el modpack";
		} finally {
			loading = false;
		}
	}

	// ── File picker ───────────────────────────────────────────────────────────
	async function selectMrpackFile() {
		try {
			const selected = await openDialog({
				multiple: false,
				filters: [{ name: "Modpacks", extensions: ["mrpack"] }],
			});
			if (selected) {
				mrpackPath = selected;
				loadPackInfo();
			}
		} catch (e) {
			console.error("Error selecting file:", e);
		}
	}

	// ── Reset ─────────────────────────────────────────────────────────────────
	function resetState() {
		name = "";
		selectedVersion = "";
		selectedIcon = null;
		versions = [];
		error = null;
		parsing = false;
		packInfo = null;
		loading = false;
		currentStep = 0;
		contentSource = "version";
	}

	function reset() {
		open = false;
		mrpackPath = null;
		resetState();
	}
</script>

<ModalBase
	bind:open
	title={t("createInstance.title")}
	width="700px"
	onclose={reset}
>
	<!-- Step indicator -->
	<div class="step-indicator">
		{#each { length: TOTAL_STEPS } as _, i}
			{@const active = i === currentStep}
			{@const done = i < currentStep}
			<div class="step-dot" class:active class:done>
				{#if done}
					<svg
						width="12"
						height="12"
						viewBox="0 0 24 24"
						fill="none"
						stroke="currentColor"
						stroke-width="3"
						stroke-linecap="round"
						stroke-linejoin="round"
					>
						<polyline points="20 6 9 17 4 12"></polyline>
					</svg>
				{:else}
					<span>{i + 1}</span>
				{/if}
			</div>
			{#if i < TOTAL_STEPS - 1}
				<div class="step-line" class:done></div>
			{/if}
		{/each}
	</div>

	<!-- Error -->
	{#if error}
		<div class="step-error">{error}</div>
	{/if}

	<!-- Step content -->
	<div class="step-content">
		<!-- Step 1: Name + Icon -->
		{#if currentStep === 0}
			<div class="step1-layout">
				<div class="icon-column">
					<span class="input-label"
						>{t("createInstance.iconLabel")}</span
					>
					<div class="icon-preview">
						{#if selectedIcon}
							<img src={selectedIcon} alt="Logo" />
							<button
								type="button"
								class="icon-clear"
								onclick={() => (selectedIcon = null)}
								title="Quitar icono"
							>
								<svg
									width="10"
									height="10"
									viewBox="0 0 24 24"
									fill="none"
									stroke="currentColor"
									stroke-width="3"
									stroke-linecap="round"
									stroke-linejoin="round"
								>
									<line x1="18" y1="6" x2="6" y2="18"></line>
									<line x1="6" y1="6" x2="18" y2="18"></line>
								</svg>
							</button>
						{:else}
							<svg
								width="28"
								height="28"
								viewBox="0 0 24 24"
								fill="none"
								stroke="currentColor"
								stroke-width="1.5"
								stroke-linecap="round"
								stroke-linejoin="round"
								style="color: var(--text-secondary); opacity: 0.4;"
							>
								<rect
									x="3"
									y="3"
									width="18"
									height="18"
									rx="2"
									ry="2"
								></rect>
								<circle cx="8.5" cy="8.5" r="1.5"></circle>
								<polyline points="21 15 16 10 5 21"></polyline>
							</svg>
						{/if}
					</div>
					<div class="icon-grid">
						{#each INSTANCE_LOGOS as iconName (iconName)}
							{@const iconPath = `/images/instances/${iconName}`}
							<button
								type="button"
								class="icon-option"
								class:selected={selectedIcon === iconPath}
								onclick={() =>
									(selectedIcon =
										selectedIcon === iconPath
											? null
											: iconPath)}
								title={iconName}
							>
								<img src={iconPath} alt={iconName} />
							</button>
						{/each}
					</div>
				</div>
				<div class="fields-column">
					<div class="input-group">
						<span class="input-label"
							>{t("createInstance.nameLabel")}</span
						>
						<input
							type="text"
							class="text-input"
							class:error={nameMsg}
							maxlength={16}
							bind:value={name}
							placeholder={t("createInstance.namePlaceholder")}
							disabled={loading}
							oninput={() => (nameMsg = null)}
							onkeydown={(e) => e.key === "Enter" && nextStep()}
						/>
						{#if nameMsg}
							<span class="input-error">{t(nameMsg)}</span>
						{/if}
					</div>
				</div>
			</div>
		{/if}

		<!-- Step 2: Version or Modpack -->
		{#if currentStep === 1}
			<div class="step2-layout">
				{#if !mrpackPath}
					<div class="source-toggle">
						<button
							type="button"
							class="source-btn"
							class:active={contentSource === "version"}
							onclick={() => {
								contentSource = "version";
								error = null;
							}}
						>
							<svg
								width="16"
								height="16"
								viewBox="0 0 24 24"
								fill="none"
								stroke="currentColor"
								stroke-width="2"
								stroke-linecap="round"
								stroke-linejoin="round"
							>
								<path
									d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"
								></path>
								<polyline points="7 10 12 15 17 10"></polyline>
								<line x1="12" y1="15" x2="12" y2="3"></line>
							</svg>
							Version instalada
						</button>
						<button
							type="button"
							class="source-btn"
							class:active={contentSource === "modpack"}
							onclick={() => {
								contentSource = "modpack";
								error = null;
							}}
						>
							<svg
								width="16"
								height="16"
								viewBox="0 0 24 24"
								fill="none"
								stroke="currentColor"
								stroke-width="2"
								stroke-linecap="round"
								stroke-linejoin="round"
							>
								<path
									d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"
								></path>
								<polyline points="17 8 12 3 7 8"></polyline>
								<line x1="12" y1="3" x2="12" y2="15"></line>
							</svg>
							Importar modpack
						</button>
					</div>
				{/if}

				{#if contentSource === "version" && !mrpackPath}
					<div class="version-section">
						<Select
							label={t("createInstance.versionLabel")}
							bind:value={selectedVersion}
							options={versionOptions}
							disabled={loading || versions.length === 0}
							placeholder={t("createInstance.noVersionsErr")}
						/>
					</div>
				{:else}
					<div class="modpack-section">
						{#if parsing}
							<div class="parsing-state">
								<p>{t("createInstance.parsingPack")}</p>
							</div>
						{:else if !mrpackPath}
							<div class="drop-zone">
								<p>{t("createInstance.dragOrDrop")}</p>
								<span class="drop-or"
									>{t("createInstance.or")}</span
								>
								<button
									type="button"
									class="btn-secondary"
									onclick={selectMrpackFile}
								>
									{t("createInstance.selectFile")}
								</button>
							</div>
						{:else if packInfo}
							<div class="pack-info">
								<div class="info-row">
									<span class="info-label">Pack</span>
									<span class="info-value"
										>{getPackName(packInfo)}</span
									>
								</div>
								<div class="info-row">
									<span class="info-label">Versión</span>
									<span class="info-value"
										>{getPackVersion(packInfo)}</span
									>
								</div>
								{#if getPackSummary(packInfo)}
									<div class="info-row">
										<span class="info-label"
											>Descripción</span
										>
										<span class="info-value summary"
											>{getPackSummary(packInfo)}</span
										>
									</div>
								{/if}
								<div class="info-row">
									<span class="info-label">Minecraft</span>
									<span class="info-value"
										>{getPackMcVersion(packInfo) ??
											"—"}</span
									>
								</div>
								<div class="info-row">
									<span class="info-label">Loader</span>
									<span class="info-value"
										>{getPackLoader(packInfo) ??
											"Vanilla"}{getPackLoaderVersion(
											packInfo,
										)
											? " " +
												getPackLoaderVersion(packInfo)
											: ""}</span
									>
								</div>
								<div class="info-row">
									<span class="info-label">Formato</span>
									<span class="info-value">Modrinth</span>
								</div>
								<div class="info-row">
									<span class="info-label">Archivos</span>
									<span class="info-value"
										>{getPackFileCount(packInfo)} mods/archivos</span
									>
								</div>
							</div>
							<button
								type="button"
								class="btn-change-file"
								onclick={selectMrpackFile}
							>
								Cambiar archivo
							</button>
						{:else if error}
							<div class="drop-zone">
								<p>{t("createInstance.dragOrDrop")}</p>
								<span class="drop-or"
									>{t("createInstance.or")}</span
								>
								<button
									type="button"
									class="btn-secondary"
									onclick={selectMrpackFile}
								>
									{t("createInstance.selectFile")}
								</button>
							</div>
						{/if}
					</div>
				{/if}
			</div>
		{/if}
	</div>

	{#snippet footer()}
		<button
			type="button"
			class="btn-secondary"
			onclick={currentStep > 0 ? prevStep : reset}
			disabled={loading}
		>
			{currentStep > 0
				? t("createInstance.back")
				: t("createInstance.cancel")}
		</button>
		{#if !isLastStep()}
			<button
				type="button"
				class="btn-primary"
				onclick={nextStep}
				disabled={loading}
			>
				{t("createInstance.next")}
			</button>
		{:else}
			<button
				type="button"
				class="btn-primary"
				onclick={handleFinalAction}
				disabled={loading ||
					(contentSource === "modpack" &&
						(!mrpackPath || !name.trim())) ||
					(contentSource === "version" && !selectedVersion)}
			>
				{loading
					? contentSource === "modpack"
						? t("createInstance.importingBtn")
						: t("createInstance.creatingBtn")
					: contentSource === "modpack"
						? t("createInstance.importBtn")
						: t("createInstance.createBtn")}
			</button>
		{/if}
	{/snippet}
</ModalBase>

<style>
	/* ── Step indicator ────────────────────────────────────────────────── */
	.step-indicator {
		display: flex;
		align-items: center;
		gap: 0;
		padding: 4px 0 12px;
	}

	.step-dot {
		width: 28px;
		height: 28px;
		border-radius: 50%;
		background: rgba(var(--accent-rgb, 255, 255, 255), 0.06);
		border: 2px solid var(--border);
		display: flex;
		align-items: center;
		justify-content: center;
		font-size: 0.7rem;
		font-weight: 700;
		color: var(--text-secondary);
		transition: all 0.2s;
		flex-shrink: 0;
	}

	.step-dot.active {
		border-color: var(--accent);
		color: var(--accent);
		background: rgba(var(--accent-rgb, 255, 255, 255), 0.12);
	}

	.step-dot.done {
		border-color: var(--accent);
		background: var(--accent);
		color: var(--accent-text, #0c0c0c);
	}

	.step-line {
		flex: 1;
		height: 2px;
		background: var(--border);
		margin: 0 8px;
		transition: background 0.2s;
	}

	.step-line.done {
		background: var(--accent);
	}

	/* ── Error ─────────────────────────────────────────────────────────── */
	.step-error {
		color: var(--color-error);
		font-size: 0.8rem;
		background: rgba(var(--color-error-rgb), 0.1);
		border: 1px solid rgba(var(--color-error-rgb), 0.2);
		border-radius: 6px;
		padding: 10px;
		text-align: center;
		font-weight: 500;
	}

	/* ── Step content ──────────────────────────────────────────────────── */
	.step-content {
		min-height: 200px;
	}

	/* ── Step 1: Name + Icon ───────────────────────────────────────────── */
	.step1-layout {
		display: flex;
		gap: 24px;
		align-items: flex-start;
	}

	.icon-column {
		display: flex;
		flex-direction: column;
		gap: 10px;
		align-items: center;
		flex-shrink: 0;
	}

	.icon-preview {
		width: 80px;
		height: 80px;
		border-radius: 12px;
		background: rgba(var(--accent-rgb, 255, 255, 255), 0.03);
		border: 2px dashed var(--border);
		display: flex;
		align-items: center;
		justify-content: center;
		position: relative;
		overflow: visible;
	}

	.icon-preview img {
		width: 56px;
		height: 56px;
		object-fit: contain;
		filter: drop-shadow(0 2px 4px rgba(0, 0, 0, 0.3));
	}

	.icon-clear {
		position: absolute;
		top: -5px;
		right: -5px;
		width: 22px;
		height: 22px;
		border-radius: 50%;
		background: var(--color-error);
		color: white;
		border: 2px solid var(--bg-card);
		cursor: pointer;
		display: flex;
		align-items: center;
		justify-content: center;
		box-shadow: var(--shadow-sm);
		transition:
			transform 0.15s,
			background 0.15s;
		opacity: 0;
	}

	.icon-preview:hover .icon-clear {
		opacity: 1;
	}

	.icon-clear:hover {
		transform: scale(1.15);
		filter: brightness(0.8);
	}

	.icon-clear:active {
		transform: scale(0.95);
	}

	.icon-grid {
		display: grid;
		grid-template-columns: repeat(3, 1fr);
		gap: 6px;
	}

	.icon-option {
		width: 40px;
		height: 40px;
		border-radius: 8px;
		background: rgba(var(--accent-rgb, 255, 255, 255), 0.03);
		border: 2px solid var(--border);
		cursor: pointer;
		padding: 6px;
		transition:
			border-color 0.15s,
			background 0.15s;
	}

	.icon-option:hover {
		background: rgba(var(--accent-rgb, 255, 255, 255), 0.06);
		border-color: var(--text-secondary);
	}

	.icon-option.selected {
		border-color: var(--accent);
		background: rgba(var(--accent-rgb, 255, 255, 255), 0.1);
	}

	.icon-option img {
		width: 100%;
		height: 100%;
		object-fit: contain;
	}

	.fields-column {
		flex: 1;
		display: flex;
		flex-direction: column;
		gap: 14px;
		padding-top: 24px;
	}

	/* ── Step 2: Version / Modpack ─────────────────────────────────────── */
	.step2-layout {
		display: flex;
		flex-direction: column;
		gap: 16px;
	}

	.source-toggle {
		display: flex;
		gap: 8px;
	}

	.source-btn {
		flex: 1;
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 8px;
		padding: 12px 16px;
		background: rgba(var(--accent-rgb, 255, 255, 255), 0.03);
		border: 2px solid var(--border);
		border-radius: var(--border-radius-sm);
		color: var(--text-secondary);
		font-size: 0.85rem;
		font-weight: 600;
		cursor: pointer;
		transition: all 0.15s;
	}

	.source-btn:hover {
		background: rgba(var(--accent-rgb, 255, 255, 255), 0.06);
		border-color: var(--text-secondary);
	}

	.source-btn.active {
		border-color: var(--accent);
		color: var(--text-primary);
		background: rgba(var(--accent-rgb, 255, 255, 255), 0.08);
	}

	.version-section {
		padding: 8px 0;
	}

	/* ── Modpack: Drop zone ────────────────────────────────────────────── */
	.parsing-state,
	.drop-zone {
		padding: 28px 16px;
		text-align: center;
		color: var(--text-secondary);
		font-size: 0.9rem;
	}

	.drop-zone {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 10px;
		border: 2px dashed var(--border);
		border-radius: var(--border-radius-sm);
	}

	.drop-or {
		font-size: 0.7rem;
		text-transform: uppercase;
		opacity: 0.5;
	}

	/* ── Modpack: Pack info ────────────────────────────────────────────── */
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

	/* ── Shared input styles ───────────────────────────────────────────── */
	.input-group {
		margin-top: 4px;
	}

	.input-group :global(.text-input.error) {
		border-color: var(--color-error) !important;
		box-shadow: 0 0 0 1px var(--color-error) !important;
	}

	.input-error {
		font-size: 0.7rem;
		color: var(--color-error);
		margin-top: 4px;
		display: block;
	}

	.input-label {
		font-size: 0.7rem;
		font-weight: 600;
		color: var(--text-secondary);
		text-transform: uppercase;
		letter-spacing: 0.5px;
		margin-bottom: 5px;
		display: block;
	}
</style>
