<script lang="ts">
	import { onMount, onDestroy } from "svelte";
	import { fade } from "svelte/transition";
	import { open as openDialog } from "@tauri-apps/plugin-dialog";
	import ModalBase from "#lib/components/layout/ModalBase.svelte";
	import Select from "#lib/components/layout/Select.svelte";
	import Icon from "#lib/icons/Icon.svelte";
	import Loading from "#lib/icons/Loading.svelte";
	import ChevronDownIcon from "#lib/icons/ChevronDownIcon.svelte";
	import ChevronRightIcon from "#lib/icons/ChevronRightIcon.svelte";
	import {
		listModpackVersions,
		previewModpackUpdate,
		applyModpackUpdate,
		cancelModpackUpdate,
		type PackSummary,
		type PackVersion,
		type PackUpdatePreview,
		type PackResolutions,
	} from "#lib/api/modpackApi.js";
	import { refreshInstanceMods } from "#lib/api/launcherService.js";
	import { t } from "#lib/i18n/index.js";
	import { createModpackUpdateSession } from "#lib/utils/modpackUpdateSession.js";
	import { animDuration } from "#lib/utils/animations.js";

	let {
		open = $bindable(),
		id,
		pack,
		busy = false,
		onupdated,
	}: {
		open: boolean;
		id: string;
		pack: PackSummary;
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
	const changeKinds = [
		{ kind: "added", marker: "+" },
		{ kind: "changed", marker: "~" },
		{ kind: "removed", marker: "−" },
	] as const;
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
	const remoteSourceIcon = $derived(
		pack.source === "curseforge" ? "brand:curseforge" : "brand:modrinth",
	);
	const contentFadeDuration = $derived(animDuration(150));
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
	animateResize={!!preview}
	hideScrollbar
	closeDisabled={working}
>
	{#if applied}
		<p role="status">{t("modpack.updated")}</p>
	{:else}
		{#if !preview}
			<fieldset
				class="source-panel"
				disabled={working || busy}
				in:fade={{ duration: contentFadeDuration }}
			>
				<legend>{t("modpack.updateSource")}</legend>
				<div class="source-options">
					{#if pack.source !== "local" && pack.project_id}
						<label
							class="source-option"
							class:selected={source === "remote"}
						>
							<input
								type="radio"
								bind:group={source}
								value="remote"
								onchange={clearPreview}
							/>
							<Icon name={remoteSourceIcon} size={18} />
							<span>
								<strong>{t("modpack.publishedVersion")}</strong>
								<small>{pack.source}</small>
							</span>
						</label>
					{/if}
					<label
						class="source-option"
						class:selected={source === "local"}
					>
						<input
							type="radio"
							bind:group={source}
							value="local"
							onchange={clearPreview}
						/>
						<Icon name="instance:folder" size={18} />
						<span>
							<strong>{t("modpack.localFile")}</strong>
							<small>.mrpack / .zip</small>
						</span>
					</label>
				</div>
				{#if source === "remote"}
					<div class="source-control">
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
						<button
							type="button"
							class="btn-secondary reload-btn"
							disabled={loadingVersions}
							onclick={loadVersions}
						>
							<Icon name="ui:refresh" size={14} />
							<span>{t("modpack.retry")}</span>
						</button>
					</div>
					{#if !loadingVersions && !versions.length}<p>
							{t("modpack.noVersions")}
						</p>{/if}
				{:else}
					<div class="source-control">
						<button
							type="button"
							class="btn-secondary"
							onclick={chooseFile}
							>{t("modpack.chooseFile")}</button
						>
						{#if path}<p class="path">{path}</p>{/if}
					</div>
				{/if}
			</fieldset>
		{/if}
		{#if preview}
			<div class="preview" in:fade={{ duration: contentFadeDuration }}>
				<div class="preview-head">
					<div class="preview-title">
						<span class="preview-name">{preview.name}</span>
						<div class="preview-actions">
							<span class="source-chip">
								<Icon
									name={source === "remote"
										? remoteSourceIcon
										: "instance:folder"}
									size={13}
								/>
								{source === "remote"
									? t("modpack.publishedVersion")
									: t("modpack.localFile")}
							</span>
							<button
								type="button"
								class="change-source"
								onclick={clearPreview}
								disabled={working}
							>
								<Icon name="nav:edit" size={13} />
								{t("modpack.changeSource")}
							</button>
						</div>
					</div>
					<div class="version-diff">
						<div class="version-diff-row">
							<Icon name="brand:vanilla" size={18} />
							<span class="version-diff-label">Minecraft</span>
							<strong>{pack.game_version}</strong>
							<span class="diff-arrow" aria-hidden="true">→</span>
							<strong>{preview.game_version}</strong>
						</div>
						<div class="version-diff-row">
							<Icon name="instance:puzzle" size={18} />
							<span class="version-diff-label"
								>{t("modpack.title")}</span
							>
							<strong>{pack.version}</strong>
							<span class="diff-arrow" aria-hidden="true">→</span>
							<strong>{preview.version}</strong>
						</div>
					</div>
				</div>
				<div class="diff-summary">
					{#each changeKinds as { kind, marker } (kind)}
						{@const files =
							preview[kind as "added" | "removed" | "changed"]}
						<details
							class="diff-group"
							class:added={kind === "added"}
							class:changed={kind === "changed"}
							class:removed={kind === "removed"}
							class:empty={files.length === 0}
						>
							<summary>
								<span class="diff-marker" aria-hidden="true"
									>{marker}</span
								>
								<span class="group-label"
									>{t(`modpack.${kind}`)}</span
								>
								<span class="count">{files.length}</span>
								<span class="summary-chevron" aria-hidden="true"
									><ChevronRightIcon size={12} /></span
								>
							</summary>
							{#if files.length}
								<ul>
									{#each files as file (file)}<li>
											{file}
										</li>{/each}
								</ul>
							{/if}
						</details>
					{/each}
				</div>
				<div class="conflicts-block">
					<div class="conflicts-head">
						<span class="group-label">{t("modpack.conflicts")}</span
						>
						<span class="count">{preview.conflicts.length}</span>
					</div>
					{#if preview.conflicts.length}
						<p class="hint">{t("modpack.conflictsHint")}</p>
						<div class="conflicts">
							{#each preview.conflicts as file (file)}
								<label class="conflict">
									<span class="conflict-file">{file}</span>
									<span class="select-wrap">
										<select
											disabled={working || busy}
											value={resolutions[file] ?? ""}
											onchange={(event) => {
												resolutions = {
													...resolutions,
													[file]: event.currentTarget
														.value as
														"keep" | "replace",
												};
											}}
										>
											<option value="" disabled
												>{t("modpack.resolve")}</option
											>
											<option value="keep"
												>{t("modpack.keep")}</option
											>
											<option value="replace"
												>{t("modpack.replace")}</option
											>
										</select>
										<span
											class="select-chevron"
											aria-hidden="true"
											><ChevronDownIcon size={12} /></span
										>
									</span>
								</label>
							{/each}
						</div>
					{/if}
				</div>
			</div>
		{/if}
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
				{#if working}
					<Loading size={14} />
				{:else}
					<span
						>{t(
							preview ? "modpack.apply" : "modpack.process",
						)}</span
					>
				{/if}
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
	.source-panel {
		gap: 10px;
	}
	.source-options {
		display: grid;
		grid-template-columns: repeat(2, minmax(0, 1fr));
		gap: 8px;
	}
	.source-option {
		min-width: 0;
		padding: 10px;
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		background: var(--bg-card);
		transition:
			background var(--transition-fast) ease,
			border-color var(--transition-fast) ease;
	}
	.source-option:hover:not(.selected) {
		background: var(--surface-hover);
	}
	.source-option.selected {
		border-color: var(--accent);
		background: rgba(var(--accent-rgb), 0.08);
		color: var(--text-primary);
	}
	.source-option span {
		display: flex;
		min-width: 0;
		flex-direction: column;
		gap: 2px;
	}
	.source-option strong,
	.source-option small {
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.source-option strong {
		font-size: var(--font-size-control);
		font-weight: var(--font-weight-medium);
	}
	.source-option small {
		color: var(--text-muted);
		font-size: var(--font-size-sm);
		text-transform: capitalize;
	}
	.source-control {
		display: flex;
		align-items: end;
		gap: 8px;
		min-width: 0;
	}
	.source-control :global(.custom-select-container) {
		flex: 1;
		min-width: 0;
	}
	.reload-btn {
		height: var(--control-min-height, 38px);
		display: inline-flex;
		align-items: center;
		gap: 6px;
		white-space: nowrap;
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

	.preview {
		display: flex;
		flex-direction: column;
		gap: 14px;
	}
	.preview-head {
		display: flex;
		flex-direction: column;
		gap: 10px;
		padding: 12px;
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		background: var(--bg-card);
	}
	.preview-title {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 10px;
	}
	.preview-actions {
		display: flex;
		align-items: center;
		gap: 6px;
		flex-shrink: 0;
	}
	.preview-name {
		color: var(--text-primary);
		font-size: var(--font-size-lg);
		font-weight: var(--font-weight-medium);
	}
	.source-chip {
		display: inline-flex;
		align-items: center;
		gap: 5px;
		flex-shrink: 0;
		padding: 3px 7px;
		border-radius: 999px;
		background: var(--surface-hover);
		color: var(--text-secondary);
		font-size: var(--font-size-sm);
	}
	.change-source {
		display: inline-flex;
		align-items: center;
		gap: 4px;
		border: none;
		border-radius: var(--border-radius-sm);
		padding: 4px 6px;
		background: transparent;
		color: var(--text-secondary);
		font: inherit;
		font-size: var(--font-size-sm);
		cursor: pointer;
	}
	.change-source:hover:not(:disabled) {
		background: var(--surface-hover);
		color: var(--text-primary);
	}
	.version-diff {
		display: grid;
		gap: 6px;
	}
	.version-diff-row {
		display: grid;
		grid-template-columns: 18px minmax(80px, 1fr) auto auto auto;
		align-items: center;
		gap: 7px;
		min-width: 0;
		color: var(--text-secondary);
		font-size: var(--font-size-sm);
	}
	.version-diff-label {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.version-diff-row strong {
		color: var(--text-primary);
		font-variant-numeric: tabular-nums;
	}
	.diff-arrow {
		color: var(--text-muted);
	}

	.diff-summary {
		display: flex;
		flex-direction: column;
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		overflow: hidden;
		background: var(--bg-card);
	}
	details.diff-group {
		border-bottom: 1px solid var(--border);
	}
	details.diff-group:last-child {
		border-bottom: none;
	}
	details.diff-group.empty {
		opacity: 0.6;
	}
	summary {
		display: flex;
		align-items: center;
		gap: 6px;
		padding: 10px;
		list-style: none;
		cursor: pointer;
		color: var(--text-primary);
		font-size: var(--font-size-sm);
		font-weight: var(--font-weight-medium);
	}
	summary::-webkit-details-marker {
		display: none;
	}
	summary:hover .group-label {
		color: var(--text-primary);
	}
	.summary-chevron {
		display: flex;
		color: var(--text-secondary);
		transition: transform var(--transition-fast) ease;
	}
	details[open] .summary-chevron {
		transform: rotate(90deg);
	}
	.diff-marker {
		width: 18px;
		height: 18px;
		display: grid;
		place-items: center;
		border-radius: 50%;
		font-size: 0.82rem;
		font-weight: 700;
	}
	.diff-group.added .diff-marker {
		background: rgba(var(--color-success-rgb), 0.14);
		color: var(--color-success);
	}
	.diff-group.changed .diff-marker {
		background: rgba(var(--accent-rgb), 0.14);
		color: var(--accent);
	}
	.diff-group.removed .diff-marker {
		background: rgba(var(--color-error-rgb), 0.14);
		color: var(--color-error);
	}
	.group-label {
		min-width: 0;
		color: var(--text-secondary);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.count {
		margin-left: auto;
		padding: 1px 8px;
		border-radius: 999px;
		background: var(--surface-hover);
		color: var(--text-secondary);
		font-size: 0.72rem;
		font-weight: 600;
		font-variant-numeric: tabular-nums;
	}
	.diff-group ul {
		max-height: 200px;
		overflow-y: auto;
		margin: 0;
		padding: 0 10px 10px;
		list-style: none;
	}
	.diff-group li {
		padding: 5px 0;
		border-top: 1px solid var(--border);
		color: var(--text-secondary);
		font-size: var(--font-size-sm);
	}
	@media (max-width: 540px) {
		.source-options {
			grid-template-columns: 1fr;
		}
		.source-control {
			align-items: stretch;
			flex-direction: column;
		}
		.preview-title {
			align-items: flex-start;
			flex-direction: column;
		}
		.preview-actions {
			width: 100%;
			justify-content: space-between;
		}
	}

	.conflicts-block {
		display: flex;
		flex-direction: column;
		gap: 8px;
	}
	.conflicts-head {
		display: flex;
		align-items: center;
		gap: 8px;
		color: var(--text-primary);
		font-size: var(--font-size-control);
		font-weight: var(--font-weight-medium);
	}
	.hint {
		margin: 0;
		color: var(--text-secondary);
		font-size: var(--font-size-sm);
	}
	.conflicts {
		display: flex;
		flex-direction: column;
		gap: 8px;
		max-height: 260px;
		overflow-y: auto;
	}
	.conflict {
		justify-content: space-between;
		gap: 12px;
		padding: 10px 12px;
		border: var(--border-width) solid var(--border);
		border-radius: var(--border-radius-sm);
		background: var(--bg-card);
	}
	.conflict-file {
		min-width: 0;
		color: var(--text-secondary);
		font-size: var(--font-size-sm);
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
	.error {
		color: var(--color-error);
	}
</style>
