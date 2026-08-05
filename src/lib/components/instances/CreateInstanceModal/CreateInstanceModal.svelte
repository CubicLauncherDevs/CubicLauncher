<script lang="ts">
	import {
		createInstance,
		uploadCustomIcon,
		fetchAll,
		parseMrpack,
		installMrpack,
		searchModrinth,
		addToQueue,
		downloadFabric,
		downloadForge,
		downloadNeoForge,
		downloadQuilt,
	} from "$lib/api/cubicApi";
	import {
		isVersionInstalled,
		loadInstalledVersions,
		invalidateInstalledVersions,
	} from "$lib/state/versionsState.svelte";
	import ModalBase from "$lib/components/layout/ModalBase.svelte";
	import { t } from "$lib/i18n";
	import type { MrpackInfo } from "$lib/types/types";
	import IconPicker from "./IconPicker.svelte";
	import VersionSelectorStep from "./VersionSelectorStep.svelte";
	import StepIndicator from "./StepIndicator.svelte";
	import PackInfo from "./PackInfo.svelte";
	import ModrinthModpackBrowser from "./ModrinthModpackBrowser.svelte";
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

	type Tab = "manual" | "import" | "modrinth";
	let tab = $state<Tab>("manual");
	let manualStep = $state(0);

	// ── Instance fields ─────────────────────────────────────────────────────────
	let name = $state("");
	let selectedIcon = $state<string | null>(null);
	let customIconPath = $state<string | null>(null);

	// ── Version selector ──────────────────────────────────────────────────────
	let selectedLoader = $state("vanilla");
	let selectedMcVersion = $state("");
	let selectedLoaderVersion = $state("");

	const finalVersionId = $derived.by(() => {
		if (selectedLoader === "vanilla") {
			return selectedMcVersion;
		}
		if (selectedLoader === "fabric" && selectedLoaderVersion) {
			return `fabric-loader-${selectedLoaderVersion}-${selectedMcVersion}`;
		}
		if (selectedLoader === "quilt" && selectedLoaderVersion) {
			return `quilt-loader-${selectedLoaderVersion}-${selectedMcVersion}`;
		}
		if (selectedLoader === "forge" && selectedLoaderVersion) {
			return `${selectedMcVersion}-forge-${selectedLoaderVersion}`;
		}
		if (selectedLoader === "neoforge" && selectedLoaderVersion) {
			return `${selectedMcVersion}-neoforge-${selectedLoaderVersion}`;
		}
		return "";
	});

	// ── Modpack (import) ──────────────────────────────────────────────────────
	let packInfo = $state<MrpackInfo | null>(null);
	let parsing = $state(false);

	// ── Common ──────────────────────────────────────────────────────────────────
	let loading = $state(false);
	let error = $state<string | null>(null);
	let existingNames = $state<string[]>([]);
	let nameMsg = $state<string | null>(null);

	const FORBIDDEN_CHARS = ["/", "\\", "<", ">", ":", '"', "|", "?", "*"];
	const MAX_NAME_LEN = 16;

	function isValidInstanceName(name: string): boolean {
		const trimmed = name.trim();
		if (!trimmed) return false;
		if (trimmed.length > MAX_NAME_LEN) return false;
		if (!/^[\0-\x7F]*$/.test(trimmed)) return false;
		if (trimmed.includes("..")) return false;
		if (trimmed.split("").some((c) => FORBIDDEN_CHARS.includes(c)))
			return false;
		return true;
	}

	function sanitizeInstanceName(name: string): string {
		let clean = name
			.normalize("NFD")
			.replace(/[\u0300-\u036f]/g, "")
			.replace(/[^\x20-\x7E]/g, "")
			.replace(/[\\/<>:"|?*]/g, "")
			.replace(/\.\./g, "")
			.replace(/\s+/g, " ")
			.trim();
		if (!clean) clean = "modpack";
		if (clean.length > MAX_NAME_LEN) clean = clean.slice(0, MAX_NAME_LEN);
		return clean;
	}

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
		if (!isValidInstanceName(trimmed)) {
			nameMsg = "createInstance.nameInvalidChars";
			return false;
		}
		nameMsg = null;
		return true;
	}

	// ── Effects ─────────────────────────────────────────────────────────────────
	let namesCache: string[] | null = null;

	$effect(() => {
		if (open) {
			nameMsg = null;
			tab = mrpackPath ? "import" : "manual";
			if (!namesCache) fetchInstances();
		}
	});

	$effect(() => {
		if (open && mrpackPath) {
			loadPackInfo();
		}
	});

	$effect(() => {
		if (open && tab === "manual" && selectedLoader) {
			const icon = selectIconForLoader(selectedLoader);
			if (icon && !selectedIcon) {
				selectedIcon = icon;
			}
		}
	});

	// ── Fetch instances ─────────────────────────────────────────────────────────
	async function fetchInstances() {
		const instances = await fetchAll();
		namesCache = instances.map((i) => i.name);
		existingNames = namesCache;
	}

	// ── Load pack info ─────────────────────────────────────────────────────────
	async function loadPackInfo() {
		if (!mrpackPath) return;
		parsing = true;
		error = null;
		try {
			const info = await parseMrpack(mrpackPath);
			if (info) {
				packInfo = info;
				if (!name.trim()) {
					name = isValidInstanceName(info.name)
						? info.name
						: sanitizeInstanceName(info.name);
				}
				const loaderIcon = selectIconForLoader(info.loader);
				if (loaderIcon) selectedIcon = loaderIcon;
			} else {
				error = "No se pudo leer el archivo .mrpack";
			}
		} finally {
			parsing = false;
		}
	}

	// ── Helpers ─────────────────────────────────────────────────────────────────
	function selectIconForLoader(loader: string | null): string | null {
		if (!loader) return null;
		const l = loader.toLowerCase();
		if (l === "fabric") return "/images/instances/fabric.png";
		if (l === "forge") return "/images/instances/forge.png";
		if (l === "neoforge" || l === "neo")
			return "/images/instances/neoforged.png";
		if (l === "quilt") return "/images/instances/vanilla.png";
		return null;
	}

	function updateIconForLoader() {
		const icon = selectIconForLoader(selectedLoader);
		if (icon && !selectedIcon) {
			selectedIcon = icon;
		}
	}

	// ── Import modpack (local) ─────────────────────────────────────────────────
	async function selectMrpackFile() {
		try {
			const selected = await openDialog({
				multiple: false,
				filters: [{ name: "Modpacks", extensions: ["mrpack"] }],
			});
			if (selected) {
				mrpackPath = selected;
				await loadPackInfo();
			}
		} catch (e) {
			console.error("Error selecting file:", e);
		}
	}

	// ── Step navigation ─────────────────────────────────────────────────────────
	function handleNext() {
		if (!validateName()) return;
		manualStep = 1;
	}

	// ── Create / Import ─────────────────────────────────────────────────────────
	async function handleFinalAction() {
		if (tab === "import" && mrpackPath && packInfo) {
			await handleImport();
		} else if (tab === "manual") {
			await handleManualCreate();
		}
	}

	function handleIconUpload(filePath: string) {
		customIconPath = filePath;
		selectedIcon = filePath;
	}

	async function handleManualCreate() {
		if (!validateName()) return;
		if (!finalVersionId) {
			error = t("createInstance.noVersionsErr");
			return;
		}
		loading = true;
		error = null;
		try {
			await createInstance(
				name,
				finalVersionId,
				customIconPath ? null : selectedIcon,
				async (uuid: string) => {
					if (customIconPath) {
						await uploadCustomIcon(uuid, customIconPath);
					}
					await enqueueSelectedVersion();
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

	async function enqueueSelectedVersion() {
		await loadInstalledVersions();
		if (isVersionInstalled(finalVersionId)) return;

		if (selectedLoader === "vanilla") {
			await addToQueue(finalVersionId);
		} else if (selectedLoader === "fabric") {
			await downloadFabric(selectedMcVersion, selectedLoaderVersion);
		} else if (selectedLoader === "quilt") {
			await downloadQuilt(selectedMcVersion, selectedLoaderVersion);
		} else if (selectedLoader === "forge") {
			await downloadForge(selectedMcVersion, selectedLoaderVersion);
		} else if (selectedLoader === "neoforge") {
			await downloadNeoForge(selectedMcVersion, selectedLoaderVersion);
		}

		invalidateInstalledVersions();
	}

	async function handleImport() {
		if (!mrpackPath || !name.trim()) return;
		loading = true;
		error = null;
		try {
			let iconUrl: string | undefined;
			try {
				const searchResult = await searchModrinth(
					name.trim(),
					"",
					undefined,
					null,
					"downloads",
					1,
					0,
					"modpack",
				);
				if (searchResult && searchResult.hits.length > 0) {
					iconUrl = searchResult.hits[0].icon_url ?? undefined;
				}
			} catch {
				/* ignore search errors */
			}

			const result = await installMrpack(
				mrpackPath,
				name.trim(),
				iconUrl,
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

	// ── Reset ───────────────────────────────────────────────────────────────────
	function resetState() {
		name = "";
		selectedLoader = "vanilla";
		selectedMcVersion = "";
		selectedLoaderVersion = "";
		selectedIcon = null;
		customIconPath = null;
		error = null;
		parsing = false;
		packInfo = null;
		loading = false;
		mrpackPath = null;
		tab = "manual";
		manualStep = 0;
	}

	function reset() {
		open = false;
		mrpackPath = null;
		resetState();
	}

	$effect(() => {
		if (open && tab === "manual") {
			updateIconForLoader();
		}
	});
</script>

<ModalBase
	bind:open
	title={t("createInstance.title")}
	width={tab === "modrinth" ? "800px" : "700px"}
	onclose={reset}
>
	{#if error}
		<div class="step-error">{error}</div>
	{/if}

	<div class="tab-bar">
		<button
			type="button"
			class="tab-btn"
			class:active={tab === "manual"}
			onclick={() => (tab = "manual")}
		>
			{t("createInstance.manualTab")}
		</button>
		<button
			type="button"
			class="tab-btn"
			class:active={tab === "import"}
			onclick={() => (tab = "import")}
		>
			{t("createInstance.importTab")}
		</button>
		<button
			type="button"
			class="tab-btn"
			class:active={tab === "modrinth"}
			onclick={() => (tab = "modrinth")}
		>
			Modrinth
		</button>
	</div>

	<div class="step-content">
		{#if tab === "modrinth"}
			<ModrinthModpackBrowser onInstalled={reset} />
		{:else if tab === "import"}
			{#if packInfo}
				<div class="modpack-summary">
					{#if parsing}
						<div class="parsing-state">
							<p>{t("createInstance.parsingPack")}</p>
						</div>
					{:else}
						<div class="import-name-section">
							<div class="input-group">
								<span class="input-label">
									{t("createInstance.nameLabel")}
								</span>
								<input
									type="text"
									class="text-input"
									class:error={nameMsg}
									maxlength={16}
									bind:value={name}
									disabled={loading}
									oninput={() => (nameMsg = null)}
									onkeydown={(e) =>
										e.key === "Enter" &&
										handleFinalAction()}
								/>
								{#if nameMsg}
									<span class="input-error">{t(nameMsg)}</span
									>
								{/if}
							</div>
						</div>
						<PackInfo {packInfo} onChangeFile={selectMrpackFile} />
					{/if}
				</div>
			{:else}
				<div class="import-layout">
					<div class="import-empty">
						<p>{t("createInstance.selectFile")}</p>
						<button
							type="button"
							class="btn-primary"
							onclick={selectMrpackFile}
						>
							{t("createInstance.selectMrpackBtn")}
						</button>
					</div>
				</div>
			{/if}
		{:else}
			<div class="create-layout">
				<StepIndicator
					currentStep={manualStep}
					totalSteps={2}
					labels={[
						t("createInstance.stepInfo"),
						t("createInstance.stepVersion"),
					]}
				/>

				{#if manualStep === 0}
					<div class="create-header">
						<IconPicker
							bind:selectedIcon
							disabled={loading}
							onupload={handleIconUpload}
						/>
						<div class="fields-column">
							<div class="input-group">
								<span class="input-label">
									{t("createInstance.nameLabel")}
								</span>
								<input
									type="text"
									class="text-input"
									class:error={nameMsg}
									maxlength={16}
									bind:value={name}
									disabled={loading}
									oninput={() => (nameMsg = null)}
									onkeydown={(e) =>
										e.key === "Enter" && handleNext()}
								/>
								{#if nameMsg}
									<span class="input-error">{t(nameMsg)}</span
									>
								{/if}
							</div>
						</div>
					</div>
				{:else}
					<VersionSelectorStep
						bind:selectedLoader
						bind:selectedMcVersion
						bind:selectedLoaderVersion
					/>
				{/if}
			</div>
		{/if}
	</div>

	{#snippet footer()}
		<div class="footer-actions">
			<div class="footer-left">
				{#if tab === "manual" && manualStep === 1}
					<button
						type="button"
						class="btn-secondary"
						onclick={() => (manualStep = 0)}
						disabled={loading}
					>
						{t("createInstance.backBtn")}
					</button>
				{/if}
			</div>
			<div class="footer-right">
				{#if tab === "manual" && manualStep === 0}
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
						onclick={handleNext}
						disabled={loading || !name.trim()}
					>
						{t("createInstance.nextBtn")}
					</button>
				{:else if tab === "manual" && manualStep === 1}
					<button
						type="button"
						class="btn-primary"
						onclick={handleManualCreate}
						disabled={loading || !finalVersionId}
					>
						{loading
							? t("createInstance.creatingBtn")
							: t("createInstance.createBtn")}
					</button>
				{:else if tab === "import"}
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
						onclick={handleFinalAction}
						disabled={loading || !mrpackPath || !name.trim()}
					>
						{loading
							? t("createInstance.importingBtn")
							: t("createInstance.importBtn")}
					</button>
				{/if}
			</div>
		</div>
	{/snippet}
</ModalBase>

<style>
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

	.tab-bar {
		display: flex;
		gap: 4px;
		margin-bottom: 8px;
		border-bottom: 1px solid var(--border);
		padding-bottom: 8px;
	}

	.tab-btn {
		padding: 6px 16px;
		border: 1px solid transparent;
		border-radius: var(--border-radius-sm);
		background: transparent;
		color: var(--text-secondary);
		font-size: 0.78rem;
		font-weight: 600;
		cursor: pointer;
		transition:
			color 0.15s ease,
			border-color 0.15s ease,
			background 0.15s ease;
	}

	.tab-btn:hover {
		color: var(--text-primary);
		background: var(--bg-item-active);
	}

	.tab-btn.active {
		color: var(--text-primary);
		border-color: var(--accent);
		background: rgba(var(--accent-rgb), 0.08);
	}

	.step-content {
		display: flex;
		flex-direction: column;
		gap: 16px;
		min-height: 320px;
	}

	.create-layout {
		display: flex;
		flex-direction: column;
		gap: 24px;
		height: 100%;
	}

	.create-header {
		display: flex;
		gap: 24px;
		align-items: flex-start;
	}

	.fields-column {
		flex: 1;
		display: flex;
		flex-direction: column;
		gap: 14px;
		padding-top: 24px;
	}

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

	.footer-actions {
		display: flex;
		align-items: center;
		justify-content: space-between;
		width: 100%;
	}

	.footer-right {
		display: flex;
		gap: 10px;
	}

	.modpack-summary {
		display: flex;
		flex-direction: column;
		gap: 12px;
	}

	.parsing-state {
		padding: 28px 16px;
		text-align: center;
		color: var(--text-secondary);
		font-size: 0.9rem;
	}

	.import-layout {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		min-height: 250px;
	}

	.import-empty {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 16px;
		color: var(--text-secondary);
		font-size: 0.85rem;
	}

	.import-name-section {
		margin-bottom: 8px;
	}

	.import-name-section :global(.text-input) {
		width: 100%;
		padding: 8px 12px;
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		background: var(--bg-input);
		color: var(--text-primary);
		font-size: 0.82rem;
		font-family: inherit;
		outline: none;
		box-sizing: border-box;
	}

	.import-name-section :global(.text-input:focus) {
		border-color: var(--accent);
	}

	.import-name-section :global(.text-input.error) {
		border-color: var(--color-error) !important;
		box-shadow: 0 0 0 1px var(--color-error) !important;
	}
</style>
