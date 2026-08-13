<script lang="ts">
	import {
		cancelInstanceImport,
		detectInstanceZip,
		importInstanceZip,
	} from "$lib/api/cubicApi";
	import { t } from "$lib/i18n";
	import { showSuccess } from "$lib/state/state.svelte";
	import type { InstanceImportPlan } from "$lib/types/types";
	import {
		MAX_INSTANCE_NAME_LEN,
		isValidInstanceName,
	} from "$lib/utils/instanceName";
	import { open as openDialog } from "@tauri-apps/plugin-dialog";
	import { onDestroy } from "svelte";

	let {
		onImported,
		initialPath,
	}: {
		onImported?: () => void;
		initialPath?: string | null;
	} = $props();

	let filePath = $state<string | null>(null);
	let plan = $state<InstanceImportPlan | null>(null);
	let previewToken = $state("");
	let targetName = $state("");
	let loading = $state(false);
	let importing = $state(false);
	let error = $state<string | null>(null);

	async function selectFile() {
		try {
			const selected = await openDialog({
				multiple: false,
				filters: [{ name: "ZIP", extensions: ["zip"] }],
				title: t("createInstance.importInstanceSelect"),
			});
			if (selected && typeof selected === "string") {
				filePath = selected;
				await detectArchive();
			}
		} catch (e) {
			console.error("Error selecting file:", e);
		}
	}

	async function detectArchive() {
		if (!filePath) return;
		if (previewToken) {
			void cancelInstanceImport(previewToken);
		}
		loading = true;
		error = null;
		plan = null;
		previewToken = "";
		targetName = "";
		try {
			const detected = await detectInstanceZip(filePath);
			if (detected) {
				plan = detected;
				previewToken = detected.preview_token;
				targetName = detected.sanitized_name;
			} else {
				error = t("createInstance.importInstanceDetectErr");
			}
		} catch (e) {
			console.error("Error detecting instance archive:", e);
			error = t("createInstance.importInstanceDetectErr");
		} finally {
			loading = false;
		}
	}

	async function handleImport() {
		if (
			!filePath ||
			!plan ||
			!previewToken ||
			!isValidInstanceName(targetName)
		)
			return;
		importing = true;
		error = null;
		try {
			const result = await importInstanceZip(
				previewToken,
				targetName.trim(),
			);
			if (result) {
				showSuccess(
					t("createInstance.importInstanceSuccessTitle"),
					t("createInstance.importInstanceSuccessMessage"),
				);
				reset();
				onImported?.();
			} else {
				error = t("createInstance.importInstanceError");
			}
		} catch (e) {
			console.error("Error importing instance archive:", e);
			error = t("createInstance.importInstanceError");
		} finally {
			importing = false;
		}
	}

	function reset() {
		if (previewToken) {
			void cancelInstanceImport(previewToken);
		}
		filePath = null;
		plan = null;
		previewToken = "";
		targetName = "";
		error = null;
		loading = false;
		importing = false;
	}

	onDestroy(() => {
		if (previewToken) {
			void cancelInstanceImport(previewToken);
		}
	});

	$effect(() => {
		if (initialPath && filePath !== initialPath) {
			filePath = initialPath;
			detectArchive();
		}
	});
</script>

<div class="instance-import">
	{#if error}
		<div class="step-error">{error}</div>
	{/if}

	{#if !plan}
		<div class="import-empty">
			<p>{t("createInstance.importInstanceHint")}</p>
			<button
				type="button"
				class="btn-primary"
				onclick={selectFile}
				disabled={loading}
			>
				{loading
					? t("createInstance.importInstanceAnalyzing")
					: t("createInstance.importInstanceSelect")}
			</button>
		</div>
	{:else}
		<div class="import-preview">
			<div class="preview-header">
				<span class="file-name" title={filePath ?? ""}>{filePath}</span>
				<button
					type="button"
					class="btn-secondary"
					onclick={selectFile}
					disabled={importing}
				>
					{t("createInstance.importInstanceChangeFile")}
				</button>
			</div>

			<div class="format-badge">
				{t("createInstance.importInstanceDetected", {
					format: plan.format_name,
				})}
			</div>

			<div class="input-group">
				<span class="input-label">{t("createInstance.nameLabel")}</span>
				<input
					type="text"
					class="text-input"
					class:error={!isValidInstanceName(targetName)}
					maxlength={MAX_INSTANCE_NAME_LEN}
					bind:value={targetName}
					disabled={importing}
				/>
			</div>

			<div class="info-grid">
				<div class="info-row">
					<span class="info-label"
						>{t("createInstance.mcVersionLabel")}</span
					>
					<span class="info-value"
						>{plan.minecraft_version ?? "—"}</span
					>
				</div>
				<div class="info-row">
					<span class="info-label"
						>{t("createInstance.versionLabel")}</span
					>
					<span class="info-value">
						{plan.loader ?? "Vanilla"}
						{plan.loader_version ? ` ${plan.loader_version}` : ""}
					</span>
				</div>
			</div>

			{#if plan.warnings.length > 0}
				<div class="warnings">
					{#each plan.warnings as warning (warning)}
						<div class="warning-item">{warning}</div>
					{/each}
				</div>
			{/if}
		</div>

		<div class="footer-actions">
			<button
				type="button"
				class="btn-secondary"
				onclick={reset}
				disabled={importing}
			>
				{t("createInstance.cancel")}
			</button>
			<button
				type="button"
				class="btn-primary"
				onclick={handleImport}
				disabled={importing || !isValidInstanceName(targetName)}
			>
				{importing
					? t("createInstance.importInstanceImporting")
					: t("createInstance.importInstanceImportBtn")}
			</button>
		</div>
	{/if}
</div>

<style>
	.instance-import {
		display: flex;
		flex-direction: column;
		gap: 16px;
		min-height: 250px;
	}

	.step-error {
		color: var(--color-error);
		font-size: 0.8rem;
		background: rgba(var(--color-error-rgb), 0.1);
		border: 1px solid rgba(var(--color-error-rgb), 0.2);
		border-radius: 6px;
		padding: 10px;
		text-align: center;
		font-weight: 500;
	}

	.import-empty {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 16px;
		flex: 1;
		color: var(--text-secondary);
		font-size: 0.85rem;
		text-align: center;
	}

	.import-preview {
		display: flex;
		flex-direction: column;
		gap: 14px;
		flex: 1;
	}

	.preview-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 12px;
		padding: 10px 12px;
		background: rgba(var(--accent-rgb, 255, 255, 255), 0.02);
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
	}

	.file-name {
		font-size: 0.8rem;
		color: var(--text-primary);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		min-width: 0;
	}

	.format-badge {
		font-size: 0.75rem;
		font-weight: 600;
		padding: 6px 12px;
		border-radius: var(--border-radius-sm);
		background: rgba(var(--accent-rgb), 0.08);
		color: var(--text-secondary);
		align-self: flex-start;
	}

	.input-group {
		display: flex;
		flex-direction: column;
		gap: 5px;
	}

	.input-label {
		font-size: 0.7rem;
		font-weight: 600;
		color: var(--text-secondary);
		text-transform: uppercase;
		letter-spacing: 0.5px;
	}

	.text-input {
		width: 100%;
		padding: 8px 12px;
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		background: var(--bg-input);
		color: var(--text-primary);
		font-size: 0.85rem;
		font-family: inherit;
		outline: none;
		box-sizing: border-box;
	}

	.text-input:focus {
		border-color: var(--accent);
	}

	.text-input.error {
		border-color: var(--color-error);
		box-shadow: 0 0 0 1px var(--color-error);
	}

	.info-grid {
		display: grid;
		grid-template-columns: repeat(2, 1fr);
		gap: 10px;
		padding: 10px 12px;
		background: var(--bg-card);
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
	}

	.info-row {
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	.info-label {
		font-size: 0.65rem;
		font-weight: 600;
		color: var(--text-secondary);
		text-transform: uppercase;
		letter-spacing: 0.5px;
	}

	.info-value {
		font-size: 0.85rem;
		color: var(--text-primary);
	}

	.warnings {
		display: flex;
		flex-direction: column;
		gap: 6px;
	}

	.warning-item {
		font-size: 0.75rem;
		color: var(--color-warning);
		background: rgba(var(--color-warning-rgb), 0.1);
		border: 1px solid rgba(var(--color-warning-rgb), 0.2);
		border-radius: var(--border-radius-sm);
		padding: 8px 12px;
	}

	.footer-actions {
		display: flex;
		align-items: center;
		justify-content: flex-end;
		gap: 10px;
	}

	.btn-primary,
	.btn-secondary {
		padding: 8px 16px;
		border-radius: var(--border-radius-sm);
		font-size: 0.8rem;
		font-weight: 600;
		font-family: inherit;
		cursor: pointer;
		transition:
			background 0.15s,
			opacity 0.15s,
			border-color 0.15s;
	}

	.btn-primary {
		background: var(--accent);
		color: var(--accent-text);
		border: 1px solid var(--accent);
	}

	.btn-primary:hover:not(:disabled) {
		filter: brightness(1.1);
	}

	.btn-secondary {
		background: transparent;
		color: var(--text-secondary);
		border: 1px solid var(--border);
	}

	.btn-secondary:hover:not(:disabled) {
		color: var(--text-primary);
		border-color: var(--text-secondary);
	}

	button:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}
</style>
