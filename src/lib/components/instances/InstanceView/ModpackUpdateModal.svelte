<script lang="ts">
	import { onMount, onDestroy } from "svelte";
	import { open as openDialog } from "@tauri-apps/plugin-dialog";
	import ModalBase from "$lib/components/layout/ModalBase.svelte";
	import Select from "$lib/components/layout/Select.svelte";
	import ChevronDownIcon from "$lib/icons/ChevronDownIcon.svelte";
	import ChevronRightIcon from "$lib/icons/ChevronRightIcon.svelte";
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
	const versionOptions = $derived(
		versions.map((version) => ({
			value: version.id,
			label: `${version.name} (${version.version})`,
		})),
	);
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
	const canPrepare = $derived(
		!working &&
			!busy &&
			(source === "remote" ? !!versionId && !loadingVersions : !!path),
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
		if (!canPrepare) return;
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
			if (!disposed) {
				if (!applied) clearPreview();
				error = String(e);
			}
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
				<Select
					bind:value={versionId}
					options={versionOptions}
					label={t("modpack.publishedVersion")}
					placeholder={t("modpack.noVersions")}
					loading={loadingVersions}
					loadingPlaceholder={t("createInstance.loading")}
					disabled={working || busy || !versions.length}
					onchange={clearPreview}
				/>
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
					<summary
						><span class="summary-chevron" aria-hidden="true"
							><ChevronRightIcon size={12} /></span
						><span>{t(`modpack.${kind}`)} ({files.length})</span
						></summary
					>
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
						><span class="conflict-file">{file}</span>
						<span class="select-wrap"
							><select
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
								><option value="keep"
									>{t("modpack.keep")}</option
								><option value="replace"
									>{t("modpack.replace")}</option
								>
							</select>
							<span class="select-chevron" aria-hidden="true"
								><ChevronDownIcon size={12} /></span
							></span
						></label
					>
				{/each}
			</div>
		{/if}
	{/if}
	{#if working && !applied}
		<div class="processing" role="status" aria-live="polite">
			<span class="spinner" aria-hidden="true"></span>
			<span>{t("modpack.working")}</span>
		</div>
	{/if}
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
				disabled={working ||
					busy ||
					(preview ? !canApply : !canPrepare)}
				onclick={preview ? apply : prepare}
			>
				{#if working}<span class="spinner" aria-hidden="true"
					></span>{/if}
				<span>{t(preview ? "modpack.apply" : "modpack.process")}</span>
			</button>{/if}
	{/snippet}
</ModalBase>

<style>
	fieldset {
		display: flex;
		flex-direction: column;
		gap: 12px;
		border: var(--border-width) solid var(--border);
		border-radius: var(--border-radius-sm);
		padding: 16px;
		margin: 0;
	}
	legend {
		padding: 0 6px;
		font-size: var(--font-size-label);
		font-weight: var(--font-weight-medium);
		text-transform: uppercase;
		letter-spacing: 1px;
		color: var(--text-secondary);
	}
	label {
		display: flex;
		align-items: center;
		gap: 10px;
		cursor: pointer;
		color: var(--text-secondary);
		font-size: var(--font-size-control);
	}
	input[type="radio"] {
		appearance: none;
		-webkit-appearance: none;
		width: 16px;
		height: 16px;
		margin: 0;
		flex-shrink: 0;
		position: relative;
		background: var(--surface-input);
		border: var(--border-width) solid var(--border);
		border-radius: 50%;
		cursor: pointer;
		transition:
			background var(--transition-fast) ease,
			border-color var(--transition-fast) ease;
	}
	input[type="radio"]:hover:not(:disabled) {
		border-color: var(--border-hover);
	}
	input[type="radio"]:checked {
		background: var(--accent);
		border-color: var(--accent);
	}
	input[type="radio"]:checked::after {
		content: "";
		position: absolute;
		top: 50%;
		left: 50%;
		width: 6px;
		height: 6px;
		border-radius: 50%;
		background: var(--accent-text);
		transform: translate(-50%, -50%);
	}
	input[type="radio"]:disabled {
		opacity: var(--disabled-opacity, 0.5);
		cursor: not-allowed;
	}
	.select-wrap {
		position: relative;
		display: inline-flex;
		flex-shrink: 0;
	}
	select {
		appearance: none;
		-webkit-appearance: none;
		background: var(--surface-input);
		color: var(--text-primary);
		border: var(--border-width) solid var(--border);
		border-radius: var(--border-radius-sm);
		padding: 8px 30px 8px 10px;
		font: inherit;
		font-size: var(--font-size-control);
		max-width: 100%;
		cursor: pointer;
		transition:
			background var(--transition-fast) ease,
			border-color var(--transition-fast) ease;
	}
	select:hover:not(:disabled) {
		background: var(--surface-hover);
		border-color: var(--border-hover);
	}
	select:disabled {
		opacity: var(--disabled-opacity, 0.5);
		cursor: not-allowed;
	}
	option {
		background: var(--bg-card);
		color: var(--text-primary);
	}
	.select-chevron {
		position: absolute;
		right: 8px;
		top: 50%;
		transform: translateY(-50%);
		display: flex;
		color: var(--text-secondary);
		pointer-events: none;
	}
	.path,
	li,
	.conflict-file {
		overflow-wrap: anywhere;
	}
	ul,
	.conflicts {
		max-height: 240px;
		overflow-y: auto;
	}
	ul {
		margin: 0 0 10px;
		padding-left: 18px;
		color: var(--text-secondary);
		font-size: var(--font-size-sm);
	}
	li::marker {
		color: var(--text-muted);
	}
	details {
		border: var(--border-width) solid var(--border);
		border-radius: var(--border-radius-sm);
		padding: 0 12px;
	}
	summary {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 10px 0;
		list-style: none;
		cursor: pointer;
		color: var(--text-primary);
		font-size: var(--font-size-control);
		font-weight: var(--font-weight-medium);
	}
	summary::-webkit-details-marker {
		display: none;
	}
	summary:hover {
		color: var(--accent);
	}
	.summary-chevron {
		display: flex;
		color: var(--text-secondary);
		transition: transform var(--transition-fast) ease;
	}
	details[open] .summary-chevron {
		transform: rotate(90deg);
	}
	.conflict {
		justify-content: space-between;
		padding: 8px 0;
	}
	.conflict-file {
		min-width: 0;
	}
	h3,
	h4 {
		color: var(--text-primary);
		font-weight: var(--font-weight-medium);
	}
	h3 {
		font-size: var(--font-size-lg);
	}
	h4 {
		font-size: var(--font-size-control);
	}
	.processing {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 12px 14px;
		border: var(--border-width) solid var(--border);
		border-radius: var(--border-radius-sm);
		background: var(--surface-card);
		color: var(--text-secondary);
		font-size: var(--font-size-control);
	}
	.processing .spinner {
		color: var(--accent);
	}
	.btn-primary {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: 8px;
	}
	.btn-primary:disabled,
	.btn-secondary:disabled {
		opacity: var(--disabled-opacity, 0.5);
		cursor: not-allowed;
	}
	.spinner {
		width: 14px;
		height: 14px;
		flex-shrink: 0;
		border: 2px solid color-mix(in srgb, currentColor 30%, transparent);
		border-top-color: currentColor;
		border-radius: 50%;
		animation: spin 0.8s linear infinite;
	}
	@keyframes spin {
		to {
			transform: rotate(360deg);
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.spinner {
			animation: none;
		}
	}
	.error {
		color: var(--color-error);
	}
</style>
