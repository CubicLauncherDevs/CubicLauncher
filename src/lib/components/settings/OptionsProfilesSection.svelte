<script lang="ts">
	import { onMount } from "svelte";
	import { ask, open } from "@tauri-apps/plugin-dialog";
	import { t } from "#lib/i18n/index.js";
	import Icon from "#lib/icons/Icon.svelte";
	import {
		createOptionsProfile,
		updateOptionsProfile,
		deleteOptionsProfile,
		importOptionsProfileFromFile,
		type OptionsProfile,
	} from "#lib/api/optionsProfiles.js";
	import {
		optionsProfilesState,
		loadOptionsProfiles,
	} from "#lib/state/optionsProfilesState.svelte.js";
	import { showSuccess, showErrorParsed } from "#lib/state/state.svelte.js";
	import OptionsVisualEditor from "./OptionsVisualEditor.svelte";
	import SegmentedControl from "./SegmentedControl.svelte";
	import {
		OPTION_PRESETS,
		OPTION_BY_KEY,
		parseOptions,
		presetContent,
	} from "#lib/utils/minecraftOptions.js";

	type EditorMode = "visual" | "advanced";

	let editing = $state<{
		id: string | null;
		name: string;
		content: string;
	} | null>(null);
	let editorMode = $state<EditorMode>("visual");
	let busy = $state(false);

	const profiles = $derived(optionsProfilesState.profiles);
	const loading = $derived(optionsProfilesState.loading);
	const editorModeOptions = $derived([
		{ value: "visual", label: t("optionsProfiles.editor.visual") },
		{ value: "advanced", label: t("optionsProfiles.editor.advanced") },
	]);
	const createPresets = $derived(
		OPTION_PRESETS.filter((preset) => preset.id !== "vanilla"),
	);

	onMount(() => {
		void loadOptionsProfiles();
	});

	function startCreate(presetId?: string) {
		editing = {
			id: null,
			name: presetId ? t(`optionsProfiles.presets.${presetId}.name`) : "",
			content: presetContent(presetId ?? "vanilla"),
		};
		editorMode = "visual";
	}

	function startEdit(profile: OptionsProfile) {
		editing = {
			id: profile.id,
			name: profile.name,
			content: profile.content,
		};
		editorMode = "visual";
	}

	function profileOptionCount(content: string): number {
		let count = 0;
		for (const key of parseOptions(content).keys()) {
			if (OPTION_BY_KEY.has(key)) count++;
		}
		return count;
	}

	function cancelEdit() {
		editing = null;
	}

	function setEditorMode(value: string) {
		editorMode = value === "advanced" ? "advanced" : "visual";
	}

	async function save() {
		if (!editing || busy) return;
		const name = editing.name.trim();
		if (!name) {
			showErrorParsed(
				JSON.stringify({
					code: "GENERIC",
					params: { error: t("optionsProfiles.nameRequired") },
				}),
			);
			return;
		}
		busy = true;
		try {
			if (editing.id) {
				await updateOptionsProfile(editing.id, name, editing.content);
				showSuccess(
					t("optionsProfiles.title"),
					t("optionsProfiles.updated"),
				);
			} else {
				await createOptionsProfile(name, editing.content);
				showSuccess(
					t("optionsProfiles.title"),
					t("optionsProfiles.created"),
				);
			}
			editing = null;
			await loadOptionsProfiles(true);
		} catch (e) {
			showErrorParsed(e);
		} finally {
			busy = false;
		}
	}

	async function remove(profile: OptionsProfile) {
		if (busy) return;
		const confirmed = await ask(
			t("optionsProfiles.deleteConfirmDesc", { name: profile.name }),
			{
				title: t("optionsProfiles.deleteConfirmTitle"),
				kind: "warning",
				okLabel: t("optionsProfiles.delete"),
				cancelLabel: t("common.cancel"),
			},
		);
		if (!confirmed) return;
		busy = true;
		try {
			await deleteOptionsProfile(profile.id);
			showSuccess(
				t("optionsProfiles.title"),
				t("optionsProfiles.deleted"),
			);
			if (editing?.id === profile.id) editing = null;
			await loadOptionsProfiles(true);
		} catch (e) {
			showErrorParsed(e);
		} finally {
			busy = false;
		}
	}

	async function duplicate(profile: OptionsProfile) {
		if (busy) return;
		busy = true;
		try {
			await createOptionsProfile(
				t("optionsProfiles.copyName", { name: profile.name }),
				profile.content,
			);
			showSuccess(
				t("optionsProfiles.title"),
				t("optionsProfiles.created"),
			);
			await loadOptionsProfiles(true);
		} catch (e) {
			showErrorParsed(e);
		} finally {
			busy = false;
		}
	}

	async function importFile() {
		if (busy) return;
		try {
			const path = await open({
				multiple: false,
				title: t("optionsProfiles.importFromFile"),
				filters: [{ name: "options.txt", extensions: ["txt"] }],
			});
			if (typeof path !== "string") return;
			busy = true;
			await importOptionsProfileFromFile(path, "");
			showSuccess(
				t("optionsProfiles.title"),
				t("optionsProfiles.created"),
			);
			await loadOptionsProfiles(true);
		} catch (e) {
			showErrorParsed(e);
		} finally {
			busy = false;
		}
	}

	function formatDate(seconds: number): string {
		if (!seconds) return "";
		return new Date(seconds * 1000).toLocaleString();
	}
</script>

<div class="profiles">
	<p class="hint">{t("optionsProfiles.description")}</p>

	<div class="actions">
		<button
			type="button"
			class="btn primary"
			disabled={busy}
			onclick={() => startCreate()}
		>
			<Icon name="nav:create" size={16} />
			{t("optionsProfiles.newProfile")}
		</button>
		<button
			type="button"
			class="btn secondary"
			disabled={busy}
			onclick={importFile}
		>
			<Icon name="ui:download" size={16} />
			{t("optionsProfiles.importFromFile")}
		</button>
	</div>

	{#if !editing}
		<div class="presets">
			<span class="label">{t("optionsProfiles.presetsLabel")}</span>
			<div class="preset-chips">
				{#each createPresets as preset (preset.id)}
					<button
						type="button"
						class="chip"
						disabled={busy}
						title={t(
							`optionsProfiles.presets.${preset.id}.description`,
						)}
						onclick={() => startCreate(preset.id)}
					>
						{t(`optionsProfiles.presets.${preset.id}.name`)}
					</button>
				{/each}
			</div>
		</div>
	{/if}

	{#if loading && profiles.length === 0}
		<div class="profiles-skeleton sk-delayed" aria-hidden="true">
			{#each { length: 4 } as _, index (index)}
				<div class="profile-skeleton-row">
					<div class="sk-stack profile-skeleton-text">
						<span class="sk-block sk-line" style="width: 42%"
						></span>
						<span
							class="sk-block sk-line sk-line-sm"
							style="width: 26%"
						></span>
					</div>
					<div class="profile-skeleton-actions">
						<span
							class="sk-block"
							style="width: 72px; height: 28px;"
						></span>
						<span
							class="sk-block"
							style="width: 68px; height: 28px;"
						></span>
						<span
							class="sk-block"
							style="width: 64px; height: 28px;"
						></span>
					</div>
				</div>
			{/each}
		</div>
		<span class="sk-sr-only" role="status">{t("common.loading")}</span>
	{:else if profiles.length === 0 && !editing}
		<p class="hint">{t("optionsProfiles.empty")}</p>
	{/if}

	{#if profiles.length > 0}
		<ul class="list">
			{#each profiles as profile (profile.id)}
				<li class="item">
					<div class="item-info">
						<span class="item-name">{profile.name}</span>
						<span class="item-meta">
							{t("optionsProfiles.optionCount", {
								count: profileOptionCount(profile.content),
							})}
							{#if profile.updated_at}
								· {formatDate(profile.updated_at)}
							{/if}
						</span>
					</div>
					<div class="item-actions">
						<button
							type="button"
							class="btn secondary"
							disabled={busy}
							title={t("optionsProfiles.duplicate")}
							onclick={() => duplicate(profile)}
						>
							{t("optionsProfiles.duplicate")}
						</button>
						<button
							type="button"
							class="btn secondary"
							disabled={busy}
							onclick={() => startEdit(profile)}
						>
							{t("common.edit")}
						</button>
						<button
							type="button"
							class="btn danger"
							disabled={busy}
							onclick={() => remove(profile)}
						>
							{t("optionsProfiles.delete")}
						</button>
					</div>
				</li>
			{/each}
		</ul>
	{/if}

	{#if editing}
		<div class="editor">
			<div class="field">
				<label class="label" for="profile-name"
					>{t("optionsProfiles.name")}</label
				>
				<input
					id="profile-name"
					type="text"
					class="input"
					maxlength="48"
					bind:value={editing.name}
					placeholder={t("optionsProfiles.namePlaceholder")}
				/>
			</div>

			<div class="editor-toolbar">
				<span class="label">{t("optionsProfiles.editor.title")}</span>
				<SegmentedControl
					label={t("optionsProfiles.editor.title")}
					value={editorMode}
					options={editorModeOptions}
					disabled={busy}
					onchange={setEditorMode}
				/>
			</div>

			{#if editorMode === "visual"}
				<OptionsVisualEditor
					bind:content={editing.content}
					disabled={busy}
				/>
			{:else}
				<div class="field">
					<label class="label" for="profile-content"
						>{t("optionsProfiles.content")}</label
					>
					<textarea
						id="profile-content"
						class="textarea"
						bind:value={editing.content}
						placeholder={t("optionsProfiles.contentPlaceholder")}
						spellcheck="false"></textarea>
					<span class="hint"
						>{t("optionsProfiles.editor.advancedHint")}</span
					>
				</div>
			{/if}

			<div class="actions">
				<button
					type="button"
					class="btn primary"
					disabled={busy}
					onclick={save}
				>
					{editing.id
						? t("optionsProfiles.save")
						: t("optionsProfiles.create")}
				</button>
				<button
					type="button"
					class="btn secondary"
					disabled={busy}
					onclick={cancelEdit}
				>
					{t("common.cancel")}
				</button>
			</div>
		</div>
	{/if}
</div>

<style>
	.profiles {
		display: flex;
		flex-direction: column;
		gap: 12px;
	}
	.hint {
		color: var(--text-secondary);
		font-size: var(--font-size-sm, 0.8rem);
		line-height: 1.5;
		margin: 0;
	}
	.actions {
		display: flex;
		flex-wrap: wrap;
		gap: 8px;
	}
	.presets {
		display: flex;
		flex-direction: column;
		gap: 6px;
	}
	.preset-chips {
		display: flex;
		flex-wrap: wrap;
		gap: 6px;
	}
	.chip {
		padding: 6px 12px;
		background: transparent;
		border: 1px dashed var(--border);
		border-radius: 999px;
		color: var(--text-secondary);
		font: inherit;
		font-size: 0.75rem;
		cursor: pointer;
		transition:
			color 0.15s,
			border-color 0.15s;
	}
	.chip:hover:not(:disabled) {
		color: var(--text-primary);
		border-color: var(--accent);
	}
	.chip:disabled {
		opacity: var(--disabled-opacity, 0.5);
		cursor: not-allowed;
	}
	.chip:focus-visible {
		outline: 2px solid var(--accent);
		outline-offset: 2px;
	}
	.editor-toolbar {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 12px;
		flex-wrap: wrap;
	}
	.editor-toolbar > :global(.segmented-control) {
		min-width: 200px;
	}
	.btn {
		display: inline-flex;
		align-items: center;
		gap: 8px;
		padding: 8px 12px;
		background: var(--surface-input);
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		color: var(--text-primary);
		font: inherit;
		font-size: 0.78rem;
		cursor: pointer;
	}
	.btn:hover:not(:disabled) {
		background: var(--surface-hover);
		border-color: var(--accent);
	}
	.btn.primary {
		border-color: rgba(var(--accent-rgb), 0.35);
		color: var(--accent);
	}
	.btn.primary:hover:not(:disabled) {
		border-color: var(--accent);
		background: rgba(var(--accent-rgb), 0.08);
	}
	.btn.danger:hover:not(:disabled) {
		border-color: var(--color-error);
		color: var(--color-error);
	}
	.btn:disabled {
		opacity: var(--disabled-opacity, 0.5);
		cursor: not-allowed;
	}
	.btn:focus-visible {
		outline: 2px solid var(--accent);
		outline-offset: 2px;
	}
	.list {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.profiles-skeleton {
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.profile-skeleton-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 12px;
		padding: 10px 12px;
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		background: var(--surface-input);
	}

	.profile-skeleton-text {
		flex: 1;
		gap: 5px;
	}

	.profile-skeleton-actions {
		display: flex;
		gap: 6px;
		flex-shrink: 0;
	}
	.item {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 12px;
		padding: 10px 12px;
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		background: var(--surface-input);
	}
	.item-info {
		display: flex;
		flex-direction: column;
		gap: 2px;
		min-width: 0;
	}
	.item-name {
		color: var(--text-primary);
		font-size: var(--font-size-sm, 0.8rem);
		font-weight: 600;
		overflow-wrap: anywhere;
	}
	.item-meta {
		color: var(--text-muted);
		font-size: var(--font-size-label, 0.72rem);
	}
	.item-actions {
		display: flex;
		flex-wrap: wrap;
		justify-content: flex-end;
		gap: 6px;
		flex-shrink: 0;
	}
	.editor {
		display: flex;
		flex-direction: column;
		gap: 12px;
		padding: 12px;
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
	}
	.field {
		display: flex;
		flex-direction: column;
		gap: 6px;
	}
	.label {
		font-size: var(--font-size-label, 0.72rem);
		color: var(--text-secondary);
	}
	.input,
	.textarea {
		background: var(--surface-input);
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		color: var(--text-primary);
		font: inherit;
		font-size: 0.8rem;
		padding: 8px 10px;
		width: 100%;
		box-sizing: border-box;
	}
	.textarea {
		min-height: 180px;
		resize: vertical;
		font-family: var(--font-mono, monospace);
	}
	.input:focus-visible,
	.textarea:focus-visible {
		outline: 2px solid var(--accent);
		outline-offset: 1px;
	}
</style>
