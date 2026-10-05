<script lang="ts">
	import { onMount, onDestroy } from "svelte";
	import { open as openDialog } from "@tauri-apps/plugin-dialog";
	import ModalBase from "$lib/components/layout/ModalBase.svelte";
	import {
		listModpackVersions,
		previewModpackUpdate,
		applyModpackUpdate,
		cancelModpackUpdate,
		type PackState,
		type PackVersion,
		type PackUpdatePreview,
		type PackResolutions,
	} from "$lib/api/modpackApi";
	import { refreshInstanceMods } from "$lib/api/launcherService";
	import { t } from "$lib/i18n";
	import { createModpackUpdateSession } from "$lib/utils/modpackUpdateSession";

	let {
		open = $bindable(),
		id,
		pack,
		busy = false,
		onupdated,
	}: {
		open: boolean;
		id: string;
		pack: Omit<PackState, "files">;
		busy?: boolean;
		onupdated: () => Promise<void>;
	} = $props();
	let source = $state<"remote" | "local">("local");
	let versions = $state.raw<PackVersion[]>([]);
	let versionId = $state("");
	let path = $state("");
	let loadingVersions = $state(false);
	let working = $state(false);
	let error = $state("");
	let applied = $state(false);
	let preview = $state.raw<PackUpdatePreview | null>(null);
	let resolutions = $state<PackResolutions>({});
	let disposed = false;
	let request = 0;
	// The parent keys this component by instance. Capture staging ownership once.
	function createSession() {
		return createModpackUpdateSession(
			id,
			{
				preview: previewModpackUpdate,
				apply: applyModpackUpdate,
				cancel: cancelModpackUpdate,
			},
			(e) => {
				if (!disposed && open) error = String(e);
				else console.error("[Modpack] Could not release staging", e);
			},
		);
	}
	const session = createSession();
	$effect(() => {
		if (!open) session.clear();
	});
	const canApply = $derived(
		!!preview &&
			preview.conflicts.every(
				(file) =>
					resolutions[file] === "keep" ||
					resolutions[file] === "replace",
			),
	);
	onDestroy(() => {
		disposed = true;
		request++;
		session.dispose();
	});
	onMount(() => {
		if (pack.source !== "local" && pack.project_id) {
			source = "remote";
			void loadVersions();
		}
	});
	async function loadVersions() {
		clearPreview();
		loadingVersions = true;
		error = "";
		try {
			const result = await listModpackVersions(id);
			if (disposed) return;
			versions = result.filter(
				(version) => version.id !== pack.version_id,
			);
			versionId = versions[0]?.id ?? "";
		} catch (e) {
			if (!disposed) error = String(e);
		} finally {
			if (!disposed) loadingVersions = false;
		}
	}
	function clearPreview() {
		request++;
		session.clear();
		preview = null;
		resolutions = {};
		error = "";
	}
	async function chooseFile() {
		if (working) return;
		working = true;
		error = "";
		try {
			const selected = await openDialog({
				multiple: false,
				directory: false,
				filters: [{ name: "Modpack", extensions: ["mrpack", "zip"] }],
			});
			if (!disposed && selected) {
				path = selected;
				clearPreview();
			}
		} catch (e) {
			if (!disposed) error = String(e);
		} finally {
			if (!disposed) working = false;
		}
	}
	async function prepare() {
		if (working || busy || (source === "remote" ? !versionId : !path))
			return;
		clearPreview();
		const generation = request;
		working = true;
		try {
			const result = await session.preview(
				source === "remote" ? versionId : null,
				source === "local" ? path : null,
			);
			if (!disposed && generation === request) preview = result;
		} catch (e) {
			if (!disposed && generation === request) error = String(e);
		} finally {
			if (!disposed && generation === request) working = false;
		}
	}
	async function apply() {
		if (!preview || !canApply || working || busy || applied) return;
		working = true;
		error = "";
		try {
			const instanceId = id;
			await session.apply(preview.token, { ...resolutions });
			refreshInstanceMods(instanceId);
			if (disposed) return;
			applied = true;
			await onupdated();
		} catch (e) {
			if (!disposed) error = String(e);
		} finally {
			if (!disposed) working = false;
		}
	}
</script>

<ModalBase
	bind:open
	title={t("modpack.update")}
	width="700px"
	closeDisabled={working}
>
	{#if applied}
		<p role="status">{t("modpack.updated")}</p>
	{:else}
		<fieldset disabled={working || busy}>
			<legend>{t("modpack.updateSource")}</legend>
			{#if pack.source !== "local" && pack.project_id}
				<label
					><input
						type="radio"
						bind:group={source}
						value="remote"
						onchange={clearPreview}
					/>{t("modpack.publishedVersion")}</label
				>
			{/if}
			<label
				><input
					type="radio"
					bind:group={source}
					value="local"
					onchange={clearPreview}
				/>{t("modpack.localFile")}</label
			>
			{#if source === "remote"}
				<label
					>{t("modpack.publishedVersion")}
					<select
						bind:value={versionId}
						onchange={clearPreview}
						disabled={loadingVersions || !versions.length}
					>
						{#each versions as version (version.id)}<option
								value={version.id}
								>{version.name} ({version.version})</option
							>{/each}
					</select>
				</label>
				{#if loadingVersions}<p role="status">
						{t("createInstance.loading")}
					</p>
				{:else if !versions.length}<p>{t("modpack.noVersions")}</p>{/if}
				<button
					type="button"
					class="btn-secondary"
					disabled={loadingVersions}
					onclick={loadVersions}>{t("modpack.retry")}</button
				>
			{:else}
				<button type="button" class="btn-secondary" onclick={chooseFile}
					>{t("modpack.chooseFile")}</button
				>
				{#if path}<p class="path">{path}</p>{/if}
			{/if}
			<button
				type="button"
				class="btn-primary"
				disabled={source === "remote"
					? !versionId || loadingVersions
					: !path}
				onclick={prepare}>{t("modpack.preview")}</button
			>
		</fieldset>
		{#if preview}
			<h3>
				{preview.name} · {preview.version} · {t(
					"modpack.installationVersion",
				)}: {preview.game_version}
			</h3>
			{#each ["added", "removed", "changed"] as kind (kind)}
				{@const files =
					preview[kind as "added" | "removed" | "changed"]}
				<details>
					<summary>{t(`modpack.${kind}`)} ({files.length})</summary>
					<ul>
						{#each files as file (file)}<li>{file}</li>{/each}
					</ul>
				</details>
			{/each}
			<h4>{t("modpack.conflicts")} ({preview.conflicts.length})</h4>
			{#if preview.conflicts.length}<p>
					{t("modpack.conflictsHint")}
				</p>{/if}
			<div class="conflicts">
				{#each preview.conflicts as file (file)}
					<label class="conflict"
						><span>{file}</span><select
							disabled={working || busy}
							value={resolutions[file] ?? ""}
							onchange={(event) => {
								resolutions = {
									...resolutions,
									[file]: event.currentTarget.value as
										"keep" | "replace",
								};
							}}
						>
							<option value="" disabled
								>{t("modpack.resolve")}</option
							><option value="keep">{t("modpack.keep")}</option
							><option value="replace"
								>{t("modpack.replace")}</option
							>
						</select></label
					>
				{/each}
			</div>
		{/if}
	{/if}
	{#if working}<p role="status">{t("modpack.working")}</p>{/if}
	{#if error}<p role="alert" class="error">{error}</p>{/if}
	{#snippet footer()}
		<button
			type="button"
			class="btn-secondary"
			disabled={working}
			onclick={() => (open = false)}
			>{t(applied ? "modpack.close" : "createInstance.cancel")}</button
		>
		{#if !applied}<button
				type="button"
				class="btn-primary"
				disabled={!canApply || working || busy}
				onclick={apply}>{t("modpack.apply")}</button
			>{/if}
	{/snippet}
</ModalBase>

<style>
	fieldset {
		display: flex;
		flex-direction: column;
		gap: 12px;
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		padding: 16px;
	}
	label {
		display: flex;
		align-items: center;
		gap: 10px;
	}
	select {
		background: var(--bg-input);
		color: var(--text-primary);
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		padding: 8px;
		max-width: 100%;
	}
	.path,
	li,
	.conflict span {
		overflow-wrap: anywhere;
	}
	ul,
	.conflicts {
		max-height: 240px;
		overflow-y: auto;
	}
	.conflict {
		justify-content: space-between;
		padding: 8px 0;
	}
	.conflict span {
		min-width: 0;
	}
	.conflict select {
		flex-shrink: 0;
	}
	.error {
		color: var(--color-error);
	}
	summary {
		cursor: pointer;
	}
</style>
