<script lang="ts">
	import { onMount } from "svelte";
	import { ask, open } from "@tauri-apps/plugin-dialog";
	import { t } from "$lib/i18n";
	import Icon from "$lib/icons/Icon.svelte";
	import {
		createOptionsProfile,
		updateOptionsProfile,
		deleteOptionsProfile,
		importOptionsProfileFromFile,
		type OptionsProfile,
	} from "$lib/api/optionsProfiles";
	import {
		optionsProfilesState,
		loadOptionsProfiles,
	} from "$lib/state/optionsProfilesState.svelte";
	import { showSuccess, showErrorParsed } from "$lib/state/state.svelte";

	let editing = $state<{
		id: string | null;
		name: string;
		content: string;
	} | null>(null);
	let busy = $state(false);

	const profiles = $derived(optionsProfilesState.profiles);
	const loading = $derived(optionsProfilesState.loading);

	onMount(() => {
		void loadOptionsProfiles();
	});

	function startCreate() {
		editing = { id: null, name: "", content: "" };
	}

	function startEdit(profile: OptionsProfile) {
		editing = {
			id: profile.id,
			name: profile.name,
			content: profile.content,
		};
	}

	function cancelEdit() {
		editing = null;
	}

	async function save() {
		if (!editing || busy) return;
		const name = editing.name.trim();
		if (!name) {
			showErrorParsed(
				JSON.stringify({ code: "GENERIC", params: { error: t("optionsProfiles.nameRequired") } }),
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
			showSuccess(t("optionsProfiles.title"), t("optionsProfiles.deleted"));
			if (editing?.id === profile.id) editing = null;
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
			showSuccess(t("optionsProfiles.title"), t("optionsProfiles.created"));
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
		<button type="button" class="btn" disabled={busy} onclick={startCreate}>
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

	{#if loading && profiles.length === 0}
		<p class="hint">{t("common.loading")}</p>
	{:else if profiles.length === 0 && !editing}
		<p class="hint">{t("optionsProfiles.empty")}</p>
	{/if}

	{#if profiles.length > 0}
		<ul class="list">
			{#each profiles as profile (profile.id)}
				<li class="item">
					<div class="item-info">
						<span class="item-name">{profile.name}</span>
						<span class="item-meta"
							>{formatDate(profile.updated_at)}</span
						>
					</div>
					<div class="item-actions">
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
			</div>
			<div class="actions">
				<button type="button" class="btn" disabled={busy} onclick={save}>
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
