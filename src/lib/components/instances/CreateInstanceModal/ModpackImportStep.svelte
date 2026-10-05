<script lang="ts">
	import { onDestroy, untrack } from "svelte";
	import {
		parseMrpack,
		installMrpack,
		searchModrinth,
	} from "$lib/api/cubicApi";
	import { t } from "$lib/i18n";
	import PackInfo from "./PackInfo.svelte";
	import type { MrpackInfo } from "$lib/types/types";
	import {
		MAX_INSTANCE_NAME_LEN,
		isValidInstanceName,
		sanitizeInstanceName,
	} from "$lib/utils/instanceName";
	import { open as openDialog } from "@tauri-apps/plugin-dialog";

	let {
		initialPath = null,
		name = $bindable(""),
		onImported,
		loading = $bindable(false),
	}: {
		initialPath?: string | null;
		name?: string;
		onImported?: () => void;
		loading?: boolean;
	} = $props();

	let packInfo = $state<MrpackInfo | null>(null);
	let parsing = $state(false);
	let error = $state<string | null>(null);
	let mrpackPath = $state<string | null>(null);
	let disposed = false;
	let parseGeneration = 0;
	onDestroy(() => {
		disposed = true;
		parseGeneration++;
	});

	async function loadPackInfo(path: string) {
		if (!path || loading || disposed) return;
		const generation = ++parseGeneration;
		parsing = true;
		error = null;
		packInfo = null;
		mrpackPath = null;
		try {
			const info = await parseMrpack(path);
			if (disposed || generation !== parseGeneration) return;
			if (info) {
				packInfo = info;
				mrpackPath = path;
				if (!name.trim()) {
					name = isValidInstanceName(info.name)
						? info.name
						: sanitizeInstanceName(info.name);
				}
			} else {
				error = t("modpack.readError");
			}
		} catch (e) {
			if (!disposed && generation === parseGeneration) error = String(e);
		} finally {
			if (!disposed && generation === parseGeneration) parsing = false;
		}
	}

	async function selectMrpackFile() {
		if (loading || parsing) return;
		try {
			const selected = await openDialog({
				multiple: false,
				filters: [{ name: "Modpacks", extensions: ["mrpack"] }],
			});
			if (!disposed && selected) {
				await loadPackInfo(selected);
			}
		} catch (e) {
			if (!disposed) error = String(e);
		}
	}

	async function handleImport() {
		if (
			disposed ||
			loading ||
			parsing ||
			!packInfo ||
			!mrpackPath ||
			!name.trim()
		)
			return;
		const importPath = mrpackPath;
		const importName = name.trim();
		loading = true;
		error = null;
		try {
			let iconUrl: string | undefined;
			try {
				const searchResult = await searchModrinth(
					importName,
					"",
					undefined,
					null,
					"downloads",
					1,
					0,
					"modpack",
				);
				if (searchResult && searchResult.hits.length > 0) {
					iconUrl = searchResult.hits[0].icon_url ?? undefined;
				}
			} catch {
				/* ignore search errors */
			}

			if (disposed) return;
			const result = await installMrpack(
				importPath,
				importName,
				iconUrl,
				() => {
					if (disposed) return;
					reset();
					onImported?.();
				},
				(err: unknown) => {
					if (!disposed) error = String(err);
				},
			);
			if (!disposed && !result && !error)
				error = t("createInstance.createErr");
		} catch (e) {
			if (!disposed) error = String(e);
		} finally {
			if (!disposed) loading = false;
		}
	}

	function reset() {
		packInfo = null;
		parsing = false;
		loading = false;
		error = null;
		mrpackPath = null;
	}

	$effect(() => {
		const path = initialPath;
		if (path) untrack(() => void loadPackInfo(path));
	});
</script>

<div class="modpack-import-step">
	{#if error}
		<div class="step-error">{error}</div>
	{/if}

	{#if packInfo}
		<div class="modpack-summary">
			{#if parsing}
				<div class="parsing-state">
					<p>{t("createInstance.parsingPack")}</p>
				</div>
			{:else}
				<div class="import-name-section">
					<div class="input-group">
						<span class="input-label">
							{t("createInstance.nameLabel")}
						</span>
						<input
							type="text"
							class="text-input"
							maxlength={MAX_INSTANCE_NAME_LEN}
							bind:value={name}
							disabled={loading}
							onkeydown={(e) =>
								e.key === "Enter" && handleImport()}
						/>
					</div>
				</div>
				<PackInfo {packInfo} onChangeFile={selectMrpackFile} />
			{/if}
		</div>

		<div class="import-actions">
			<button
				type="button"
				class="btn-primary"
				onclick={handleImport}
				disabled={loading || parsing || !mrpackPath || !name.trim()}
			>
				{loading
					? t("createInstance.importingBtn")
					: t("createInstance.importBtn")}
			</button>
		</div>
	{:else}
		<div class="import-layout">
			<div class="import-empty">
				<p>{t("createInstance.selectFile")}</p>
				<button
					type="button"
					class="btn-primary"
					onclick={selectMrpackFile}
					disabled={parsing}
				>
					{t("createInstance.selectMrpackBtn")}
				</button>
			</div>
		</div>
	{/if}
</div>

<style>
	.modpack-import-step {
		display: flex;
		flex-direction: column;
		gap: 12px;
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

	.modpack-summary {
		display: flex;
		flex-direction: column;
		gap: 12px;
	}

	.parsing-state {
		padding: 28px 16px;
		text-align: center;
		color: var(--text-secondary);
		font-size: 0.9rem;
	}

	.import-name-section :global(.text-input) {
		width: 100%;
		padding: 8px 12px;
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		background: var(--bg-input);
		color: var(--text-primary);
		font-size: 0.82rem;
		font-family: inherit;
		outline: none;
		box-sizing: border-box;
	}

	.import-name-section :global(.text-input:focus) {
		border-color: var(--accent);
	}

	.import-layout {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		min-height: 220px;
	}

	.import-empty {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 16px;
		color: var(--text-secondary);
		font-size: 0.85rem;
	}

	.import-actions {
		display: flex;
		justify-content: flex-end;
	}

	.btn-primary {
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
		background: var(--accent);
		color: var(--accent-text);
		border: 1px solid var(--accent);
	}

	.btn-primary:hover:not(:disabled) {
		filter: brightness(1.1);
	}

	button:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.input-group {
		margin-top: 4px;
	}

	.input-label {
		font-size: 0.7rem;
		font-weight: 600;
		color: var(--text-secondary);
		text-transform: uppercase;
		letter-spacing: 0.5px;
		margin-bottom: 5px;
		display: block;
	}
</style>
