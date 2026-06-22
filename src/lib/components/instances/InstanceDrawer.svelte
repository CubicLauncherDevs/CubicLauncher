<script lang="ts">
	import CollapsibleSection from "$lib/components/settings/CollapsibleSection.svelte";
	import { t } from "$lib/i18n";
	import Select from "$lib/components/layout/Select.svelte";
	import type { InstanceDto } from "$lib/types/types";
	import { INSTANCE_LOGOS } from "$lib/icons/logos";
	import { onMount } from "svelte";
	import { updateInst } from "$lib/api/launcherService";
	import { getInstalledVersions } from "$lib/api/cubicApi";
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
	let selectedJavaVersion = $state("");
	let instGameVersion = $state("");
	// Esto no lo veo necesario traducir ya que se autoexplica.
	let JavaOptions = [
		{
			value: "default",
			label: "Default",
			badge: t("instanceEditor.recommended"),
		},
		{ value: "jre8", label: "Java 8" },
		{ value: "jre17", label: "Java 17" },
		{ value: "jre21", label: "Java 21" },
		{ value: "jre25", label: "Java 25" },
	];
	async function handleSave() {
		saving = true;
		console.log(
			"SAVING: " +
				instanceName +
				" " +
				instGameVersion +
				" " +
				selectedIcon,
		);
		updateInst(instance.uuid, instanceName, instGameVersion, selectedIcon);
		setTimeout(() => {
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
		installedVersions = await getInstalledVersions();
		instGameVersion = instance.version;
	});
</script>

<div class="qm-root">
	<!-- Header -->
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
				<div style="margin-bottom: 4px;">
					{t("createInstance.iconLabel")}
				</div>
				<div id="icon-selector" class="icon-selector">
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
				<div class="name-section">
					<span>{t("createInstance.nameLabel")}</span>
					<input
						placeholder={t("createInstance.namePlaceholder")}
						id="name-input"
						type="text"
						class="text-input"
						bind:value={instanceName}
					/>
				</div>
				<div class="version-section">
					<Select
						value={instGameVersion}
						options={versionOptions}
						label={t("createInstance.versionLabel")}
						onchange={(v) => {
							instGameVersion = v;
							handleSave();
						}}
					/>
				</div>
			</CollapsibleSection>
			<CollapsibleSection
				title={t("settings.advanced")}
				iconSrc="/images/icons/settings.svg"
				storageKey="instance_general"
			>
				<Select
					value={selectedJavaVersion}
					options={JavaOptions}
					label={t("instanceEditor.javaVersion")}
					onchange={handleSave}
				/>
				<span class="qm-themes-hint"
					>{t("instanceEditor.javaHint")}</span
				>
				<div class="qm-field-group">
					<div class="qm-field">
						<label for="min-mem"
							>{t("settings.minecraft.minRam")}</label
						>
						<div class="qm-ram-stepper">
							<button
								type="button"
								class="qm-stepper-btn"
								onclick={() => {
									const v = minMem - 0.5;
									if (v >= 0.5) minMem = v;
								}}>−</button
							>
							<span class="qm-ram-value">{minMem} GB</span>
							<button
								type="button"
								class="qm-stepper-btn"
								onclick={() => {
									const v = minMem + 0.5;
									if (v <= maxMem) minMem = v;
								}}>+</button
							>
						</div>
					</div>
					<div class="qm-field">
						<label for="max-mem"
							>{t("settings.minecraft.maxRam")}</label
						>
						<div class="qm-ram-stepper">
							<button
								type="button"
								class="qm-stepper-btn"
								onclick={() => {
									const v = maxMem - 0.5;
									if (v >= minMem) maxMem = v;
								}}>−</button
							>
							<span class="qm-ram-value">{maxMem} GB</span>
							<button
								type="button"
								class="qm-stepper-btn"
								onclick={() => {
									const v = maxMem + 0.5;
									if (v <= 64) maxMem = v;
								}}>+</button
							>
						</div>
					</div>
				</div>
				<span class="qm-ram-hint"
					>{t("settings.minecraft.ramHint")}</span
				>
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
	/* ── Section group ───────────────────────────── */
	.section-group {
		border: 1px solid var(--border-color);
		overflow: hidden;
		margin-bottom: 16px;
	}

	.name-section {
		margin-top: 1vb;
	}

	.version-section {
		margin-top: 1vb;
	}

	.section-group :global(.cs-root) {
		border: none;
		border-bottom: 1px solid var(--border-color);
	}

	.section-group :global(.cs-root:last-child) {
		border-bottom: none;
	}

	/* ── Text input ───────────────────────────────── */
	.text-input {
		width: 100%;
		background: var(--bg-input);
		border: 1px solid var(--border-color);
		color: var(--text-primary);
		padding: 8px 12px;
		border-radius: var(--border-radius-sm);
		font-size: 0.9rem;
		box-sizing: border-box;
	}

	.text-input:focus {
		outline: none;
		border-color: var(--text-muted);
	}

	/* ── RAM stepper ──────────────────────────────── */
	.qm-field {
		margin-top: 1vw;
	}

	.qm-field label {
		display: block;
		font-size: 0.8rem;
		color: var(--text-secondary);
		margin-bottom: 6px;
	}

	.qm-field-group {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 15px;
	}

	.qm-ram-stepper {
		display: flex;
		align-items: center;
		background: var(--bg-input);
		border: 1px solid var(--border-color);
		border-radius: var(--border-radius-sm);
		box-shadow: inset 0 1px 2px rgba(0, 0, 0, 0.25);
	}

	.qm-stepper-btn {
		background: none;
		border: none;
		color: var(--text-secondary);
		padding: 8px 14px;
		font-size: 1.1rem;
		font-weight: 700;
		cursor: pointer;
		transition: color 0.15s;
		line-height: 1;
	}

	.qm-stepper-btn:hover {
		color: var(--text-primary);
	}

	.qm-ram-value {
		flex: 1;
		text-align: center;
		font-size: 0.9rem;
		font-weight: 600;
		color: var(--text-primary);
		padding: 8px 4px;
		user-select: none;
	}

	.qm-ram-hint {
		display: block;
		margin-top: 1ch;
		font-size: 0.75rem;
		color: var(--text-muted);
		line-height: 1.5;
		padding: 0 4px;
	}

	/* ── Hint text ────────────────────────────────── */
	.qm-themes-hint {
		display: block;
		margin-top: 8px;
		font-size: 0.75rem;
		color: var(--text-secondary);
		line-height: 1.4;
		cursor: pointer;
		transition: color 0.2s;
	}

	.qm-themes-hint:hover {
		color: var(--text-primary);
	}

	/* ── Save button ──────────────────────────────── */
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
