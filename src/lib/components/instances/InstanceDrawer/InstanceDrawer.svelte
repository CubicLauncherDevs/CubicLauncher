<script lang="ts">
	import CollapsibleSection from "$lib/components/settings/CollapsibleSection.svelte";
	import { t } from "$lib/i18n";
	import type { InstanceDto } from "$lib/types/types";
	import { onMount, onDestroy } from "svelte";
	import { updateInst } from "$lib/api/launcherService";
	import { getInstalledVersions, reinstallVersion } from "$lib/api/cubicApi";
	import GeneralSection from "./GeneralSection.svelte";
	import AdvancedSection from "./AdvancedSection.svelte";

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
	let instGameVersion = $state("");
	let useOverrides = $state(false);

	let JavaOptions = [
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

	async function handleSave() {
		saving = true;
		let newOverrides = useOverrides
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
			instGameVersion,
			selectedIcon,
			newOverrides,
		);
		savingTimer = setTimeout(() => {
			saving = false;
		}, 1000);
	}

	let installedVersions = $state<string[]>([]);

	let versionOptions = $derived(
		installedVersions.map((v) => ({ value: v, label: v })),
	);

	onMount(async () => {
		selectedIcon = instance.icon;
		instanceName = instance.name;
		instGameVersion = instance.version;
		installedVersions = await getInstalledVersions();
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

	function handleVersionChange(version: string) {
		instGameVersion = version;
		handleSave();
	}

	function handleJavaChange() {
		handleSave();
	}

	async function handleReinstall() {
		await reinstallVersion(instance.version);
		onclose?.();
	}
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
					bind:instGameVersion
					{versionOptions}
					{saving}
					onVersionChange={handleVersionChange}
					onReinstall={handleReinstall}
				/>
			</CollapsibleSection>
			<CollapsibleSection
				title={t("settings.advanced")}
				iconSrc="/images/icons/terminal.svg"
				storageKey="instance_general"
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
