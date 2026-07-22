<script lang="ts">
	import CollapsibleSection from "$lib/components/settings/CollapsibleSection.svelte";
	import { t } from "$lib/i18n";
	import type { InstanceDto } from "$lib/types/types";
	import { onMount, onDestroy } from "svelte";
	import { updateInst } from "$lib/api/launcherService";
	import { reinstallVersion } from "$lib/api/cubicApi";
	import GeneralSection from "./GeneralSection.svelte";
	import AdvancedSection from "./AdvancedSection.svelte";
	import InstallationSection from "./InstallationSection.svelte";

	interface Props {
		onclose?: () => void;
		instance: InstanceDto;
	}

	let { onclose, instance }: Props = $props();

	let minMem = $state(1);
	let maxMem = $state(2);
	let instanceName = $state("");
	let selectedIcon = $state<string | null>(null);
	let saving = $state(false);
	let savingTimer: ReturnType<typeof setTimeout> | undefined;

	onDestroy(() => {
		clearTimeout(savingTimer);
	});

	let selectedJavaVersion = $state("");
	let useOverrides = $state(false);

	let parsedVersion = $derived(parseInstanceVersion(instance.version));
	let selectedLoader = $state(parsedVersion.loader);
	let selectedMcVersion = $state(parsedVersion.mcVersion);
	let selectedLoaderVersion = $state(parsedVersion.loaderVersion);

	$effect(() => {
		selectedLoader = parsedVersion.loader;
		selectedMcVersion = parsedVersion.mcVersion;
		selectedLoaderVersion = parsedVersion.loaderVersion;
	});

	let repairing = $state(false);

	const JavaOptions = [
		{
			value: "default",
			label: "Default",
			badge: t("instanceEditor.recommended"),
		},
		{ value: "8", label: "Java 8" },
		{ value: "17", label: "Java 17" },
		{ value: "21", label: "Java 21" },
		{ value: "25", label: "Java 25" },
	];

	const finalVersionId = $derived.by(() => {
		if (selectedLoader === "vanilla") return selectedMcVersion;
		if (selectedLoader === "fabric" && selectedLoaderVersion)
			return `fabric-loader-${selectedLoaderVersion}-${selectedMcVersion}`;
		if (selectedLoader === "quilt" && selectedLoaderVersion)
			return `quilt-loader-${selectedLoaderVersion}-${selectedMcVersion}`;
		if (selectedLoader === "forge" && selectedLoaderVersion)
			return `${selectedMcVersion}-forge-${selectedLoaderVersion}`;
		if (selectedLoader === "neoforge" && selectedLoaderVersion)
			return `${selectedMcVersion}-neoforge-${selectedLoaderVersion}`;
		return "";
	});

	function parseInstanceVersion(version: string) {
		const fabricMatch = version.match(/^fabric-loader-([\d.]+)-(.+)$/);
		if (fabricMatch)
			return {
				loader: "fabric" as const,
				mcVersion: fabricMatch[2],
				loaderVersion: fabricMatch[1],
			};

		const quiltMatch = version.match(/^quilt-loader-([\d.]+)-(.+)$/);
		if (quiltMatch)
			return {
				loader: "quilt" as const,
				mcVersion: quiltMatch[2],
				loaderVersion: quiltMatch[1],
			};

		const neoforgeIdx = version.indexOf("-neoforge-");
		if (neoforgeIdx >= 0)
			return {
				loader: "neoforge" as const,
				mcVersion: version.substring(0, neoforgeIdx),
				loaderVersion: version.substring(neoforgeIdx + 10),
			};

		const forgeIdx = version.indexOf("-forge-");
		if (forgeIdx >= 0)
			return {
				loader: "forge" as const,
				mcVersion: version.substring(0, forgeIdx),
				loaderVersion: version.substring(forgeIdx + 7),
			};

		return {
			loader: "vanilla" as const,
			mcVersion: version,
			loaderVersion: "",
		};
	}

	async function handleSave() {
		saving = true;
		const newOverrides = useOverrides
			? {
					javaVersion:
						selectedJavaVersion && selectedJavaVersion !== "default"
							? Number(selectedJavaVersion)
							: null,
					memory: {
						minMem: Math.round(minMem * 1024),
						maxMem: Math.round(maxMem * 1024),
					},
				}
			: null;
		await updateInst(
			instance.uuid,
			instanceName,
			finalVersionId,
			selectedIcon,
			newOverrides,
		);
		savingTimer = setTimeout(() => {
			saving = false;
		}, 1000);
	}

	async function handleRepair() {
		repairing = true;
		await reinstallVersion(finalVersionId);
		repairing = false;
		onclose?.();
	}

	function handleJavaChange() {
		handleSave();
	}

	onMount(async () => {
		selectedIcon = instance.icon;
		instanceName = instance.name;
		if (instance.overrides) {
			useOverrides = true;
			selectedJavaVersion =
				instance.overrides.javaVersion != null
					? String(instance.overrides.javaVersion)
					: "default";
			minMem = (instance.overrides.memory?.minMem ?? 1024) / 1024;
			maxMem = (instance.overrides.memory?.maxMem ?? 2048) / 1024;
		}
	});
</script>

<div class="qm-root">
	<div class="qm-header">
		<span class="qm-label">{instance.name}</span>
		<button type="button" class="qm-close-btn" onclick={onclose}>✕</button>
	</div>

	<div class="qm-scroll" data-tutorial="settings-scroll">
		<div class="section-group">
			<CollapsibleSection
				title={t("instanceEditor.generalTitle")}
				iconSrc="/images/icons/settings.svg"
				storageKey="instance_general"
			>
				<GeneralSection
					bind:selectedIcon
					bind:instanceName
					instanceUuid={instance.uuid}
				/>
			</CollapsibleSection>
			<CollapsibleSection
				title="Installation"
				iconSrc="/images/icons/download.svg"
				storageKey="instance_installation"
			>
				<InstallationSection
					bind:selectedLoader
					bind:selectedMcVersion
					bind:selectedLoaderVersion
					onRepair={handleRepair}
					{repairing}
				/>
			</CollapsibleSection>
			<CollapsibleSection
				title={t("settings.advanced")}
				iconSrc="/images/icons/terminal.svg"
				storageKey="instance_advanced"
			>
				<AdvancedSection
					bind:useOverrides
					bind:selectedJavaVersion
					bind:minMem
					bind:maxMem
					javaOptions={JavaOptions}
					{saving}
					onJavaChange={handleJavaChange}
				/>
			</CollapsibleSection>
		</div>
	</div>

	<div class="save-footer">
		<button
			type="button"
			class="qm-save-btn"
			onclick={handleSave}
			disabled={saving}
		>
			{saving ? t("settings.java.saving") : t("settings.java.saveBtn")}
		</button>
	</div>
</div>

<style>
	.section-group {
		border: 1px solid var(--border-color);
		overflow: hidden;
		margin-bottom: 16px;
	}

	.section-group :global(.cs-root) {
		border: none;
		border-bottom: 1px solid var(--border-color);
	}

	.section-group :global(.cs-root:last-child) {
		border-bottom: none;
	}

	.qm-save-btn {
		width: 100%;
		background: var(--bg-card);
		color: var(--text-primary);
		border: 1px solid var(--border-color);
		padding: 10px 12px;
		border-radius: var(--border-radius-sm);
		font-family: var(--font-family);
		font-weight: 600;
		cursor: pointer;
		transition:
			background 0.15s,
			border-color 0.15s;
		box-shadow: var(--shadow-sm);
	}

	.qm-save-btn:hover:not(:disabled) {
		background: var(--bg-item-active);
		border-color: var(--border-color);
	}

	.qm-save-btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.save-footer {
		padding: 12px 20px;
		border-top: 1px solid var(--border-color);
	}
</style>
