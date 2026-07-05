<script lang="ts">
	import {
		createInstance,
		fetchAll,
		getInstalledVersions,
		parseMrpack,
		installMrpack,
	} from "$lib/api/cubicApi";
	import ModalBase from "$lib/components/layout/ModalBase.svelte";
	import { t } from "$lib/i18n";
	import { launcherStore } from "$lib/state/state.svelte";
	import type { MrpackInfo } from "$lib/types/types";
	import StepIndicator from "./StepIndicator.svelte";
	import IconPicker from "./IconPicker.svelte";
	import SourceStep from "./SourceStep.svelte";

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
	let versionsCache: string[] | null = null;
	let namesCache: string[] | null = null;

	$effect(() => {
		if (open) {
			currentStep = 0;
			contentSource = "version";
			nameMsg = null;
			if (!versionsCache) fetchVersions();
			if (!namesCache) fetchInstances();
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
		namesCache = instances.map((i) => i.name);
		existingNames = namesCache;
	}

	// ── Fetch versions ────────────────────────────────────────────────────────
	async function fetchVersions() {
		const rawVersions = await getInstalledVersions();
		versionsCache = rawVersions.sort((a, b) =>
			b.localeCompare(a, undefined, {
				numeric: true,
				sensitivity: "base",
			}),
		);
		versions = versionsCache;
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
	<StepIndicator bind:currentStep totalSteps={TOTAL_STEPS} />

	{#if error}
		<div class="step-error">{error}</div>
	{/if}

	<div class="step-content">
		{#if currentStep === 0}
			<div class="step1-layout">
				<IconPicker bind:selectedIcon disabled={loading} />
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

		{#if currentStep === 1}
			<SourceStep
				bind:contentSource
				bind:mrpackPath
				{packInfo}
				{parsing}
				{loading}
				bind:error
				{versions}
				bind:selectedVersion
				{versionOptions}
				onloadPackInfo={loadPackInfo}
			/>
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

	.step-content {
		min-height: 200px;
	}

	.step1-layout {
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

</style>
