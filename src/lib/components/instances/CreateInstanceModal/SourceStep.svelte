<script lang="ts">
	import Select from "$lib/components/layout/Select.svelte";
	import { t } from "$lib/i18n";
	import { open as openDialog } from "@tauri-apps/plugin-dialog";
	import PackInfo from "./PackInfo.svelte";
	import type { MrpackInfo } from "$lib/types/types";

	let {
		contentSource = $bindable<"version" | "modpack">("version"),
		mrpackPath = $bindable<string | null>(null),
		packInfo = null as MrpackInfo | null,
		parsing = false,
		loading = false,
		error = $bindable<string | null>(null),
		versions = [] as string[],
		selectedVersion = $bindable(""),
		versionOptions = [] as { value: string; label: string }[],
		onloadPackInfo,
	}: {
		contentSource: "version" | "modpack";
		mrpackPath: string | null;
		packInfo: MrpackInfo | null;
		parsing: boolean;
		loading: boolean;
		error: string | null;
		versions: string[];
		selectedVersion: string;
		versionOptions: { value: string; label: string }[];
		onloadPackInfo: () => void;
	} = $props();

	async function selectMrpackFile() {
		try {
			const selected = await openDialog({
				multiple: false,
				filters: [{ name: "Modpacks", extensions: ["mrpack"] }],
			});
			if (selected) {
				mrpackPath = selected;
				onloadPackInfo();
			}
		} catch (e) {
			console.error("Error selecting file:", e);
		}
	}
</script>

<div class="step2-layout">
	{#if !mrpackPath}
		<div class="source-toggle">
			<button
				type="button"
				class="source-btn"
				class:active={contentSource === "version"}
				onclick={() => {
					contentSource = "version";
					error = null;
				}}
			>
				<svg
					width="16"
					height="16"
					viewBox="0 0 24 24"
					fill="none"
					stroke="currentColor"
					stroke-width="2"
					stroke-linecap="round"
					stroke-linejoin="round"
				>
					<path
						d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"
					></path>
					<polyline points="7 10 12 15 17 10"></polyline>
					<line x1="12" y1="15" x2="12" y2="3"></line>
				</svg>
				Version instalada
			</button>
			<button
				type="button"
				class="source-btn"
				class:active={contentSource === "modpack"}
				onclick={() => {
					contentSource = "modpack";
					error = null;
				}}
			>
				<svg
					width="16"
					height="16"
					viewBox="0 0 24 24"
					fill="none"
					stroke="currentColor"
					stroke-width="2"
					stroke-linecap="round"
					stroke-linejoin="round"
				>
					<path
						d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"
					></path>
					<polyline points="17 8 12 3 7 8"></polyline>
					<line x1="12" y1="3" x2="12" y2="15"></line>
				</svg>
				Importar modpack
			</button>
		</div>
	{/if}

	{#if contentSource === "version" && !mrpackPath}
		<div class="version-section">
			<Select
				label={t("createInstance.versionLabel")}
				bind:value={selectedVersion}
				options={versionOptions}
				disabled={loading || versions.length === 0}
				placeholder={t("createInstance.noVersionsErr")}
			/>
		</div>
	{:else}
		<div class="modpack-section">
			{#if parsing}
				<div class="parsing-state">
					<p>{t("createInstance.parsingPack")}</p>
				</div>
			{:else if !mrpackPath}
				<div class="drop-zone">
					<p>{t("createInstance.dragOrDrop")}</p>
					<span class="drop-or">{t("createInstance.or")}</span>
					<button
						type="button"
						class="btn-secondary"
						onclick={selectMrpackFile}
					>
						{t("createInstance.selectFile")}
					</button>
				</div>
			{:else if packInfo}
				<PackInfo {packInfo} onChangeFile={selectMrpackFile} />
			{:else if error}
				<div class="drop-zone">
					<p>{t("createInstance.dragOrDrop")}</p>
					<span class="drop-or">{t("createInstance.or")}</span>
					<button
						type="button"
						class="btn-secondary"
						onclick={selectMrpackFile}
					>
						{t("createInstance.selectFile")}
					</button>
				</div>
			{/if}
		</div>
	{/if}
</div>

<style>
	.step2-layout {
		display: flex;
		flex-direction: column;
		gap: 16px;
	}

	.source-toggle {
		display: flex;
		gap: 8px;
	}

	.source-btn {
		flex: 1;
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 8px;
		padding: 12px 16px;
		background: rgba(var(--accent-rgb, 255, 255, 255), 0.03);
		border: 2px solid var(--border);
		border-radius: var(--border-radius-sm);
		color: var(--text-secondary);
		font-size: 0.85rem;
		font-weight: 600;
		cursor: pointer;
		transition: all 0.15s;
	}

	.source-btn:hover {
		background: rgba(var(--accent-rgb, 255, 255, 255), 0.06);
		border-color: var(--text-secondary);
	}

	.source-btn.active {
		border-color: var(--accent);
		color: var(--text-primary);
		background: rgba(var(--accent-rgb, 255, 255, 255), 0.08);
	}

	.version-section {
		padding: 8px 0;
	}

	.parsing-state,
	.drop-zone {
		padding: 28px 16px;
		text-align: center;
		color: var(--text-secondary);
		font-size: 0.9rem;
	}

	.drop-zone {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 10px;
		border: 2px dashed var(--border);
		border-radius: var(--border-radius-sm);
	}

	.drop-or {
		font-size: 0.7rem;
		text-transform: uppercase;
		opacity: 0.5;
	}
</style>
