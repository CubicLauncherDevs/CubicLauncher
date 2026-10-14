<script lang="ts">
	import { onMount, untrack } from "svelte";
	import { ask } from "@tauri-apps/plugin-dialog";
	import { t } from "$lib/i18n";
	import Select from "$lib/components/layout/Select.svelte";
	import type { InstanceDto } from "$lib/types/types";
	import { InstState } from "$lib/types/types";
	import {
		applyOptionsProfile,
		createOptionsProfileFromInstance,
		setInstanceOptionsProfile,
		optionsErrorCode,
	} from "$lib/api/optionsProfiles";
	import {
		optionsProfilesState,
		loadOptionsProfiles,
	} from "$lib/state/optionsProfilesState.svelte";
	import {
		showSuccess,
		showWarning,
		showErrorParsed,
	} from "$lib/state/state.svelte";

	let { instance }: { instance: InstanceDto } = $props();

	let selectedProfileId = $state<string>(
		untrack(() => instance.options_profile_id ?? ""),
	);
	let autoApply = $state<boolean>(untrack(() => instance.auto_apply_options));
	let busy = $state(false);
	let showSaveForm = $state(false);
	let newProfileName = $state("");
	let lastUuid = $state("");

	// Reinicializa el estado local al cambiar de instancia.
	$effect(() => {
		const uuid = instance.uuid;
		if (uuid !== lastUuid) {
			lastUuid = uuid;
			untrack(() => {
				selectedProfileId = instance.options_profile_id ?? "";
				autoApply = instance.auto_apply_options;
				showSaveForm = false;
				newProfileName = "";
			});
		}
	});

	onMount(() => {
		void loadOptionsProfiles();
	});

	const profiles = $derived(optionsProfilesState.profiles);
	const profileOptions = $derived([
		{ value: "", label: t("instanceEditor.options.none") },
		...profiles.map((p) => ({ value: p.id, label: p.name })),
	]);
	const running = $derived(instance.status !== InstState.Off);

	async function persist() {
		if (busy) return;
		busy = true;
		try {
			await setInstanceOptionsProfile(
				instance.uuid,
				selectedProfileId || null,
				autoApply,
			);
		} catch (e) {
			showErrorParsed(e);
		} finally {
			busy = false;
		}
	}

	function handleProfileChange(value: string) {
		selectedProfileId = value;
		void persist();
	}

	function handleAutoApplyChange() {
		void persist();
	}

	async function applyNow() {
		if (busy || !selectedProfileId) return;
		busy = true;
		try {
			let result;
			try {
				result = await applyOptionsProfile(
					instance.uuid,
					selectedProfileId,
					false,
				);
			} catch (e) {
				if (optionsErrorCode(e) !== "INST_OPTIONS_EXISTS") throw e;
				const confirmed = await ask(
					t("instanceEditor.options.overwriteDesc"),
					{
						title: t("instanceEditor.options.overwriteTitle"),
						kind: "warning",
						okLabel: t("instanceEditor.options.applyNow"),
						cancelLabel: t("common.cancel"),
					},
				);
				if (!confirmed) return;
				result = await applyOptionsProfile(
					instance.uuid,
					selectedProfileId,
					true,
				);
			}
			if (result.changed) {
				showSuccess(
					t("instanceEditor.options.title"),
					t("instanceEditor.options.applied"),
				);
			} else {
				showWarning(
					t("instanceEditor.options.title"),
					t("instanceEditor.options.appliedNoChange"),
				);
			}
		} catch (e) {
			showErrorParsed(e);
		} finally {
			busy = false;
		}
	}

	function startSaveForm() {
		showSaveForm = true;
		newProfileName = t("instanceEditor.options.saveCurrentDefault", {
			instance: instance.name,
		});
	}

	async function saveCurrent() {
		if (busy) return;
		const name = newProfileName.trim();
		if (!name) return;
		busy = true;
		try {
			await createOptionsProfileFromInstance(instance.uuid, name);
			showSuccess(
				t("optionsProfiles.title"),
				t("instanceEditor.options.savedFromInstance"),
			);
			showSaveForm = false;
			newProfileName = "";
			await loadOptionsProfiles(true);
		} catch (e) {
			showErrorParsed(e);
		} finally {
			busy = false;
		}
	}
</script>

<fieldset disabled={running || busy}>
	<div class="qm-field">
		<label for="instance-options-profile"
			>{t("instanceEditor.options.profile")}</label
		>
		<Select
			bind:value={selectedProfileId}
			options={profileOptions}
			onchange={handleProfileChange}
		/>
	</div>

	<div class="qm-field-checkbox">
		<input
			type="checkbox"
			id="instance-options-auto"
			bind:checked={autoApply}
			onchange={handleAutoApplyChange}
		/>
		<label for="instance-options-auto"
			>{t("instanceEditor.options.autoApply")}</label
		>
	</div>
	<span class="qm-hint">{t("instanceEditor.options.autoApplyHint")}</span>

	<div class="qm-actions">
		<button
			type="button"
			class="qm-action-btn"
			disabled={!selectedProfileId || busy}
			onclick={applyNow}
		>
			{busy
				? t("instanceEditor.options.applying")
				: t("instanceEditor.options.applyNow")}
		</button>
		<button
			type="button"
			class="qm-action-btn secondary"
			disabled={busy}
			onclick={startSaveForm}
		>
			{t("instanceEditor.options.saveCurrent")}
		</button>
	</div>

	{#if showSaveForm}
		<div class="qm-field">
			<label for="new-options-profile-name"
				>{t("instanceEditor.options.saveCurrentPrompt")}</label
			>
			<input
				id="new-options-profile-name"
				type="text"
				class="qm-input"
				maxlength="48"
				bind:value={newProfileName}
			/>
			<div class="qm-actions">
				<button
					type="button"
					class="qm-action-btn"
					disabled={busy || !newProfileName.trim()}
					onclick={saveCurrent}
				>
					{t("optionsProfiles.create")}
				</button>
				<button
					type="button"
					class="qm-action-btn secondary"
					disabled={busy}
					onclick={() => (showSaveForm = false)}
				>
					{t("common.cancel")}
				</button>
			</div>
		</div>
	{/if}
</fieldset>

<style>
	fieldset {
		border: none;
		padding: 0;
		margin: 0;
	}
	fieldset:disabled {
		opacity: 0.45;
		pointer-events: none;
	}
	.qm-field {
		margin-top: 1vw;
	}
	.qm-field label {
		display: block;
		font-size: 0.8rem;
		color: var(--text-secondary);
		margin-bottom: 6px;
	}
	.qm-field-checkbox {
		display: flex;
		align-items: center;
		gap: 12px;
		margin-top: 12px;
		cursor: pointer;
		user-select: none;
	}
	.qm-field-checkbox input[type="checkbox"] {
		appearance: none;
		-webkit-appearance: none;
		width: 18px;
		height: 18px;
		background: var(--bg-input);
		border: 1px solid var(--border-color);
		border-radius: var(--border-radius-sm);
		cursor: pointer;
		position: relative;
		transition: all 0.2s;
	}
	.qm-field-checkbox input[type="checkbox"]:checked {
		background: var(--accent);
		border-color: var(--accent);
	}
	.qm-field-checkbox input[type="checkbox"]:checked::after {
		content: "✓";
		position: absolute;
		top: 50%;
		left: 50%;
		transform: translate(-50%, -50%);
		color: var(--accent-text);
		font-size: 11px;
		font-weight: 800;
	}
	.qm-field-checkbox label {
		font-size: 0.85rem;
		color: var(--text-secondary);
		cursor: pointer;
		transition: color 0.2s;
	}
	.qm-field-checkbox:hover label {
		color: var(--text-primary);
	}
	.qm-hint {
		display: block;
		margin-top: 8px;
		font-size: 0.75rem;
		color: var(--text-muted);
		line-height: 1.5;
	}
	.qm-actions {
		display: flex;
		flex-wrap: wrap;
		gap: 8px;
		margin-top: 14px;
	}
	.qm-action-btn {
		padding: 8px 12px;
		background: var(--bg-input);
		border: 1px solid var(--border-color);
		border-radius: var(--border-radius-sm);
		color: var(--text-primary);
		font: inherit;
		font-size: 0.8rem;
		cursor: pointer;
		transition: all 0.15s;
	}
	.qm-action-btn:hover:not(:disabled) {
		border-color: var(--accent);
	}
	.qm-action-btn.secondary {
		opacity: 0.9;
	}
	.qm-action-btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}
	.qm-input {
		width: 100%;
		background: var(--bg-input);
		border: 1px solid var(--border-color);
		color: var(--text-primary);
		padding: 8px 10px;
		border-radius: var(--border-radius-sm);
		font-size: 0.85rem;
		box-sizing: border-box;
	}
	.qm-input:focus {
		outline: none;
		border-color: var(--text-muted);
	}
</style>
