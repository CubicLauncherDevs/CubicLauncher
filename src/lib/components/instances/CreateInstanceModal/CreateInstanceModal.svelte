<script lang="ts">
	import {
		createInstance,
		uploadCustomIcon,
		fetchAll,
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
	import Icon from "$lib/icons/Icon.svelte";
	import IconPicker from "./IconPicker.svelte";
	import VersionSelectorStep from "./VersionSelectorStep.svelte";
	import StepIndicator from "./StepIndicator.svelte";
	import ModrinthModpackBrowser from "./ModrinthModpackBrowser.svelte";
	import CurseForgeModpackBrowser from "./CurseForgeModpackBrowser.svelte";
	import LocalImportStep from "./LocalImportStep.svelte";
	import {
		MAX_INSTANCE_NAME_LEN,
		isValidInstanceName,
	} from "$lib/utils/instanceName";

	let {
		open = $bindable(),
		mrpackPath = $bindable<string | null>(null),
		instanceZipPath = $bindable<string | null>(null),
		oncreated,
	} = $props<{
		open: boolean;
		mrpackPath?: string | null;
		instanceZipPath?: string | null;
		oncreated?: () => void;
	}>();

	type Tab = "manual" | "modrinth" | "curseforge" | "local";
	let tab = $state<Tab>("manual");
	let manualStep = $state(0);

	const TABS: { id: Tab; label: string; icon: string }[] = [
		{
			id: "manual",
			label: t("createInstance.manualTab"),
			icon: "/images/icons/nav/create.svg",
		},
		{
			id: "modrinth",
			label: "Modrinth",
			icon: "/images/instances/modth.png",
		},
		{
			id: "curseforge",
			label: "CurseForge",
			icon: "/images/instances/forge.png",
		},
		{
			id: "local",
			label: t("createInstance.localTab"),
			icon: "/images/icons/instance/folder.svg",
		},
	];

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

	// ── Common ──────────────────────────────────────────────────────────────────
	let loading = $state(false);
	let error = $state<string | null>(null);
	let existingNames = $state<string[]>([]);
	let nameMsg = $state<string | null>(null);

	function validateName(): boolean {
		const trimmed = name.trim();
		if (!trimmed) {
			nameMsg = "createInstance.emptyNameErr";
			return false;
		}
		if (trimmed.length > MAX_INSTANCE_NAME_LEN) {
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
			if (mrpackPath || instanceZipPath) {
				tab = "local";
			} else {
				tab = "manual";
			}
			if (!namesCache) fetchInstances();
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

	// ── Step navigation ─────────────────────────────────────────────────────────
	function handleNext() {
		if (!validateName()) return;
		manualStep = 1;
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

	// ── Reset ───────────────────────────────────────────────────────────────────
	function resetState() {
		name = "";
		selectedLoader = "vanilla";
		selectedMcVersion = "";
		selectedLoaderVersion = "";
		selectedIcon = null;
		customIconPath = null;
		error = null;
		loading = false;
		mrpackPath = null;
		instanceZipPath = null;
		tab = "manual";
		manualStep = 0;
	}

	function reset() {
		open = false;
		mrpackPath = null;
		instanceZipPath = null;
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
	width={tab === "modrinth" || tab === "curseforge" ? "800px" : "700px"}
	onclose={reset}
>
	{#if error}
		<div class="step-error">{error}</div>
	{/if}

	<div class="tab-bar" role="tablist">
		{#each TABS as tabItem (tabItem.id)}
			<button
				type="button"
				class="tab-btn"
				role="tab"
				aria-selected={tab === tabItem.id}
				class:active={tab === tabItem.id}
				onclick={() => (tab = tabItem.id)}
			>
				<Icon src={tabItem.icon} size={18} />
				<span>{tabItem.label}</span>
			</button>
		{/each}
	</div>

	<div class="step-content">
		{#if tab === "modrinth"}
			<ModrinthModpackBrowser onInstalled={reset} />
		{:else if tab === "curseforge"}
			<CurseForgeModpackBrowser onInstalled={reset} />
		{:else if tab === "local"}
			<LocalImportStep
				bind:name
				onImported={reset}
				initialMrpackPath={mrpackPath}
				initialInstanceZipPath={instanceZipPath}
			/>
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
									maxlength={MAX_INSTANCE_NAME_LEN}
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
				{:else if tab === "local" || tab === "curseforge"}
					<!-- LocalImportStep y CurseForge gestionan su propio contenido -->
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
		gap: 6px;
		margin-bottom: 8px;
		border-bottom: 1px solid var(--border);
		padding-bottom: 8px;
	}

	.tab-btn {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: 6px;
		padding: 7px 14px;
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
		background: rgba(var(--accent-rgb), 0.1);
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
</style>
