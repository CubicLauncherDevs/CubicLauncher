<script lang="ts">
	import { onDestroy } from "svelte";
	import { open } from "@tauri-apps/plugin-dialog";
	import { t } from "$lib/i18n";
	import { launcherStore } from "$lib/state/state.svelte";
	import Icon from "$lib/icons/Icon.svelte";
	import {
		scanLauncherMigration,
		cancelLauncherMigration,
		migrateLauncherInstances,
		type MigrationCandidate,
		type MigrationProgress,
		type MigrationProvider,
		type MigrationResult,
	} from "$lib/api/launcherMigration";
	import { MAX_INSTANCE_NAME_LEN } from "$lib/utils/instanceName";
	import {
		suggestMigrationNames,
		validMigrationNames,
	} from "$lib/utils/migrationNames";

	let { onImported }: { onImported?: () => void } = $props();
	let provider = $state<MigrationProvider>("official");
	let rows = $state<
		(MigrationCandidate & { selected: boolean; targetName: string })[]
	>([]);
	let issues = $state<string[]>([]);
	let token = $state("");
	let scanning = $state(false);
	let importing = $state(false);
	let cancelling = $state(false);
	let scanned = $state(false);
	let location = $state("");
	let error = $state("");
	let progress = $state<MigrationProgress | null>(null);
	let results = $state<MigrationResult[]>([]);
	let destroyed = false;
	const uid = $props.id();
	const selected = $derived(rows.filter((row) => row.selected));
	const existing = $derived(
		launcherStore.loadedInstances.map((instance) => instance.name),
	);
	const valid = $derived(
		validMigrationNames(
			selected.map((row) => row.targetName),
			existing,
		),
	);
	const busy = $derived(scanning || importing);
	const percent = $derived(
		progress && progress.total > 0
			? Math.min(
					100,
					Math.round((progress.copied / progress.total) * 100),
				)
			: 0,
	);

	function releasePreview() {
		if (token) void cancelLauncherMigration(token).catch(console.error);
		token = "";
	}

	function changeProvider(next: MigrationProvider) {
		if (busy) return;
		releasePreview();
		provider = next;
		rows = [];
		issues = [];
		results = [];
		error = "";
		location = "";
		scanned = false;
	}

	async function scan(root: string | null) {
		if (busy) return;
		releasePreview();
		scanning = true;
		rows = [];
		issues = [];
		results = [];
		error = "";
		progress = null;
		location = root ?? "";
		try {
			const result = await scanLauncherMigration(provider, root);
			if (destroyed) {
				void cancelLauncherMigration(result.token).catch(console.error);
				return;
			}
			token = result.token;
			issues = result.issues;
			const names = suggestMigrationNames(
				result.candidates.map((c) => c.suggested_name),
				existing,
			);
			rows = result.candidates.map((candidate, index) => ({
				...candidate,
				selected: !candidate.error && !!candidate.version,
				targetName: names[index],
			}));
		} catch (e) {
			error = String(e);
		} finally {
			scanning = false;
			scanned = true;
		}
	}

	async function browse() {
		try {
			const path = await open({
				directory: true,
				multiple: false,
				title: t("migration.browse"),
			});
			if (!destroyed && typeof path === "string") await scan(path);
		} catch (e) {
			error = String(e);
		}
	}

	async function migrate() {
		if (!token || !valid || busy) return;
		importing = true;
		cancelling = false;
		error = "";
		try {
			results = await migrateLauncherInstances(
				token,
				selected.map((row) => ({
					id: row.id,
					name: row.targetName.trim(),
				})),
				(value) => {
					if (!destroyed) progress = value;
				},
			);
		} catch (e) {
			error = String(e);
		} finally {
			releasePreview();
			importing = false;
			cancelling = false;
		}
	}

	async function cancel() {
		if (!token || cancelling) return;
		cancelling = true;
		try {
			await cancelLauncherMigration(token);
		} catch (e) {
			error = String(e);
			cancelling = false;
		}
	}

	function versionLabel(candidate: MigrationCandidate) {
		if (!candidate.version) return "—";
		const { mc_version, loader } = candidate.version;
		return `${mc_version} · ${
			typeof loader === "string"
				? loader
				: Object.entries(loader)
						.map(([name, version]) => `${name} ${version}`)
						.join(", ")
		}`;
	}

	onDestroy(() => {
		destroyed = true;
		releasePreview();
	});
</script>

<section class="migration" aria-label={t("migration.title")}>
	<p class="hint">{t("migration.description")}</p>
	<div class="providers">
		{#each ["official", "multimc"] as choice (choice)}
			<button
				type="button"
				class="provider"
				class:active={provider === choice}
				aria-pressed={provider === choice}
				disabled={busy}
				onclick={() => changeProvider(choice as MigrationProvider)}
			>
				<Icon
					name={choice === "official"
						? "brand:vanilla"
						: "instance:folder"}
					size={22}
				/>
				<span
					><strong
						>{choice === "official"
							? t("migration.official")
							: "MultiMC (Forks)"}</strong
					><small
						>{choice === "official"
							? t("migration.officialHint")
							: "MultiMC · PolyMC · Prism Launcher"}</small
					></span
				>
			</button>
		{/each}
	</div>
	<div class="actions">
		<button
			type="button"
			class="btn-primary"
			disabled={busy}
			onclick={() => scan(null)}
			>{scanning
				? t("migration.scanning")
				: t("migration.detect")}</button
		>
		<button
			type="button"
			class="btn-secondary"
			disabled={busy}
			onclick={browse}>{t("migration.browse")}</button
		>
	</div>
	<p class="hint">
		{provider === "official"
			? t("migration.officialFolder")
			: t("migration.multimcFolder")}
	</p>
	{#if location}<p class="path">{location}</p>{/if}
	{#if error}<p class="error" role="alert">{error}</p>{/if}
	{#each issues as issue, index (index)}<p class="error">{issue}</p>{/each}

	{#if results.length}
		<div class="results" aria-live="polite">
			<h3>
				{t("migration.summary", {
					success: results.filter((r) => r.status === "success")
						.length,
					total: results.length,
				})}
			</h3>
			{#each results as result (result.id)}
				<div class="result" class:failed={result.status === "error"}>
					<strong>{result.name}</strong><span
						>{t(`migration.${result.status}`)}</span
					>
					{#if result.error && result.status !== "cancelled"}<small
							>{result.error}</small
						>{/if}
				</div>
			{/each}
			<p class="hint">{t("migration.downloadHint")}</p>
			<button type="button" class="btn-primary" onclick={onImported}
				>{t("migration.done")}</button
			>
		</div>
	{:else if rows.length}
		<div class="list-toolbar">
			<span
				>{t("migration.selected", {
					count: selected.length,
					total: rows.length,
				})}</span
			>
			<button
				type="button"
				class="btn-secondary"
				disabled={busy || !token}
				onclick={() => {
					const all = rows.every(
						(row) => row.error || !row.version || row.selected,
					);
					rows.forEach(
						(row) =>
							(row.selected =
								!all && !row.error && !!row.version),
					);
				}}>{t("migration.toggleAll")}</button
			>
		</div>
		<div class="candidates">
			{#each rows as row (row.id)}
				<div class="candidate" class:selected={row.selected}>
					<label class="candidate-title"
						><input
							type="checkbox"
							bind:checked={row.selected}
							disabled={busy ||
								!!row.error ||
								!row.version ||
								!token}
						/><strong>{row.name}</strong></label
					>
					<span class="version">{versionLabel(row)}</span>
					<p class="path" title={row.source}>{row.source}</p>
					{#if row.reinstalls_components}
						<p class="hint shared">
							{t("migration.reinstallsComponents")}
						</p>
					{/if}
					{#if row.shared_directory}<p class="hint shared">
							{t("migration.shared")}
						</p>{/if}
					{#if row.error}<p class="error">{row.error}</p>
					{:else if row.selected}
						<label class="name-label" for={`${uid}-${row.id}`}
							>{t("migration.targetName")}</label
						>
						<input
							id={`${uid}-${row.id}`}
							class="text-input"
							bind:value={row.targetName}
							maxlength={MAX_INSTANCE_NAME_LEN}
							disabled={busy || !token}
						/>
					{/if}
				</div>
			{/each}
		</div>
		{#if importing}
			<div class="progress" role="status" aria-live="polite">
				{#if progress}
					<strong
						>{t("migration.progress", {
							index: progress.index,
							count: progress.count,
						})} · {rows.find((row) => row.id === progress?.id)
							?.targetName}</strong
					>
					<span
						>{t(`migration.${progress.phase}`)}
						{progress.phase === "copying"
							? `${percent}%`
							: ""}</span
					>
					<progress
						max="100"
						value={progress.phase === "copying" &&
						progress.total > 0
							? percent
							: undefined}
						aria-label={t("migration.copying")}
					></progress>
					{#if progress.file}<small class="path"
							>{progress.file}</small
						>{/if}
				{/if}
				<button
					type="button"
					class="btn-secondary"
					disabled={cancelling}
					onclick={cancel}
					>{cancelling
						? t("migration.cancelling")
						: t("createInstance.cancel")}</button
				>
			</div>
		{:else}
			{#if selected.length && !valid}<p class="error">
					{t("migration.invalidNames")}
				</p>{/if}
			<button
				type="button"
				class="btn-primary"
				disabled={!valid || !token || busy}
				onclick={migrate}
				>{t("migration.import", { count: selected.length })}</button
			>
		{/if}
	{:else if scanned && !scanning}
		<p class="empty">{t("migration.empty")}</p>
	{/if}
</section>

<style>
	.migration {
		display: flex;
		flex-direction: column;
		gap: 12px;
		min-width: 0;
	}
	.hint {
		color: var(--text-secondary);
		font-size: var(--font-size-sm);
		line-height: 1.5;
		margin: 0;
	}
	.providers {
		display: grid;
		grid-template-columns: repeat(2, minmax(0, 1fr));
		gap: 10px;
	}
	.provider {
		display: flex;
		align-items: center;
		gap: 12px;
		padding: 14px;
		background: var(--surface-input);
		border: 1px solid var(--border);
		border-radius: var(--border-radius);
		color: var(--text-primary);
		font: inherit;
		text-align: start;
		cursor: pointer;
	}
	.provider.active {
		border-color: var(--accent);
		background: color-mix(in srgb, var(--accent) 9%, var(--surface-input));
	}
	.provider span {
		display: flex;
		flex-direction: column;
		gap: 4px;
		min-width: 0;
	}
	.provider strong {
		font-size: var(--font-size-sm);
	}
	small {
		color: var(--text-secondary);
		font-size: var(--font-size-label);
		overflow-wrap: anywhere;
	}
	.actions,
	.list-toolbar {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 8px;
	}
	.list-toolbar {
		justify-content: space-between;
		font-size: var(--font-size-sm);
		color: var(--text-secondary);
	}
	.candidates {
		display: flex;
		flex-direction: column;
		gap: 8px;
		max-height: 320px;
		overflow-y: auto;
		padding: 2px;
	}
	.candidate {
		padding: 12px;
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		background: var(--surface-input);
		display: flex;
		flex-direction: column;
		gap: 7px;
	}
	.candidate.selected {
		border-color: color-mix(in srgb, var(--accent) 50%, var(--border));
	}
	.candidate-title {
		display: flex;
		align-items: center;
		gap: 8px;
		font-size: var(--font-size-sm);
		overflow-wrap: anywhere;
		cursor: pointer;
		user-select: none;
	}
	.candidate-title:has(input:disabled) {
		cursor: not-allowed;
	}
	input[type="checkbox"] {
		appearance: none;
		-webkit-appearance: none;
		width: 18px;
		height: 18px;
		margin: 0;
		flex-shrink: 0;
		position: relative;
		background: var(--bg-input);
		border: 1px solid var(--border-color);
		border-radius: var(--border-radius-sm);
		cursor: pointer;
		transition:
			background var(--transition-fast),
			border-color var(--transition-fast);
	}
	input[type="checkbox"]:hover:not(:disabled) {
		border-color: var(--text-muted);
	}
	input[type="checkbox"]:checked {
		background: var(--accent);
		border-color: var(--accent);
	}
	input[type="checkbox"]:checked::after {
		content: "✓";
		position: absolute;
		top: 50%;
		left: 50%;
		transform: translate(-50%, -50%);
		color: var(--accent-text);
		font-size: 11px;
		font-weight: 800;
	}
	input[type="checkbox"]:disabled {
		opacity: var(--disabled-opacity, 0.5);
		cursor: not-allowed;
	}
	progress {
		accent-color: var(--accent);
	}
	.version,
	.name-label {
		font-size: var(--font-size-label);
		color: var(--text-secondary);
	}
	.path {
		overflow-wrap: anywhere;
		color: var(--text-muted);
		font-size: var(--font-size-label);
		margin: 0;
	}
	.shared {
		padding: 6px 8px;
		background: var(--surface-hover);
		border-radius: var(--border-radius-sm);
	}
	.error {
		color: var(--color-error);
		font-size: var(--font-size-sm);
		overflow-wrap: anywhere;
		margin: 0;
	}
	.empty {
		text-align: center;
		color: var(--text-secondary);
		padding: 20px;
	}
	.progress,
	.results {
		display: flex;
		flex-direction: column;
		gap: 10px;
		font-size: var(--font-size-sm);
	}
	progress {
		width: 100%;
		height: 7px;
	}
	.result {
		display: flex;
		flex-wrap: wrap;
		justify-content: space-between;
		gap: 6px;
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		padding: 10px;
	}
	.result small {
		flex-basis: 100%;
	}
	.result.failed {
		border-color: var(--color-error);
	}
	h3 {
		margin: 0;
		font-size: var(--font-size-control);
	}
	button:disabled {
		opacity: var(--disabled-opacity, 0.5);
		cursor: not-allowed;
	}
	button:focus-visible,
	input:focus-visible {
		outline: 2px solid var(--accent);
		outline-offset: 2px;
	}
	@media (max-width: 550px) {
		.providers {
			grid-template-columns: 1fr;
		}
	}
</style>
