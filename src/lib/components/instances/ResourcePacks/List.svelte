<script lang="ts">
	import {
		addInstanceFile,
		deleteInstanceFile,
		getInstanceResourcePacks,
		getInstanceShaderPacks,
	} from "$lib/api/cubicApi";
	import type { ModDto } from "$lib/types/types";
	import { SvelteSet } from "svelte/reactivity";
	import { t } from "$lib/i18n";
	import { open } from "@tauri-apps/plugin-dialog";
	import Trash from "$lib/icons/Trash.svelte";
	import Cubic from "$lib/icons/Cubic.svelte";

	let {
		supportsShaders,
		contentType,
		isLoading,
		instanceId,
		installedPackNames,
		packs,
		i18nPrefix,
		// eslint-disable-next-line no-useless-assignment
		mode = $bindable(),
	}: {
		supportsShaders: boolean;
		contentType: string | undefined;
		isLoading: boolean;
		instanceId: string;
		installedPackNames: SvelteSet<string>;
		packs: ModDto[];
		i18nPrefix: string;
		mode: string;
	} = $props();
	// ESTADOS, NO SE SI ME FALTAN MOVER ALGUNOS DE EL COMPONENTE PADRE PERO YA FUI.
	let selected = new SvelteSet<string>();
	const subDir = $derived(
		contentType === "shaders" ? "shaderpacks" : "resourcepacks",
	);
	async function loadPacks() {
		if (instanceId) {
			isLoading = true;
			packs =
				contentType === "shaders"
					? await getInstanceShaderPacks(instanceId)
					: await getInstanceResourcePacks(instanceId);
			installedPackNames.clear();
			// installedPackNames = packs.map((p) => p.name.toLowerCase());
			for (const p of packs) {
				installedPackNames.add(p.name.toLocaleLowerCase());
			}
			isLoading = false;
		}
	}

	function toggleSelect(filename: string) {
		if (selected.has(filename)) {
			selected.delete(filename);
		} else {
			selected.add(filename);
		}
	}

	async function handleBulkDelete() {
		const count = selected.size;
		if (count === 0) return;
		if (
			confirm(
				t("instanceView.deleteSelectedConfirm").replace(
					"{count}",
					String(count),
				),
			)
		) {
			for (const filename of selected) {
				await deleteInstanceFile(instanceId, subDir, filename);
			}
			selected.clear();
			await loadPacks();
		}
	}
	async function handleAdd() {
		const selected = await open({
			multiple: true,
			filters: [
				{
					name:
						contentType === "shaders"
							? "Shader Pack"
							: "Resource Pack",
					extensions: ["zip"],
				},
			],
		});
		if (selected && Array.isArray(selected)) {
			for (const path of selected) {
				await addInstanceFile(instanceId, "resourcepacks", path);
			}
			await loadPacks();
		} else if (selected && typeof selected === "string") {
			await addInstanceFile(instanceId, "resourcepacks", selected);
			await loadPacks();
		}
	}
</script>

<div class="resources-section">
	{#if supportsShaders}
		<div class="rp-subtabs">
			<button
				type="button"
				class="rp-subtab {contentType === 'resourcepacks'
					? 'active'
					: ''}"
				onclick={() => {
					contentType = "resourcepacks";
					loadPacks();
				}}
			>
				{t("instanceView.resources.title")}
			</button>
			<button
				type="button"
				class="rp-subtab {contentType === 'shaders' ? 'active' : ''}"
				onclick={() => {
					contentType = "shaders";
					loadPacks();
				}}
			>
				{t("instanceView.shaders.title")}
			</button>
		</div>
	{/if}
	<div class="section-header">
		<div class="section-header-left">
			<span class="section-title"
				>{t(i18nPrefix + ".title")} ({packs.length})</span
			>
		</div>
		<div class="section-actions">
			{#if selected.size > 0}
				<button
					type="button"
					class="delete-selected-btn"
					onclick={handleBulkDelete}
				>
					<Trash width="14" height="14" />
					{selected.size}
					{t("instanceView.deleteSelected")}
				</button>
			{/if}
			<button
				type="button"
				class="browse-btn"
				onclick={() => (mode = "browse")}
			>
				{t(i18nPrefix + ".getPacks")}
			</button>
			<button type="button" class="add-btn" onclick={handleAdd}>
				{t(i18nPrefix + ".addBtn")}
			</button>
		</div>
	</div>

	<div class="packs-grid">
		{#each packs as pack (pack.filename)}
			<div
				class="pack-card"
				class:selected={selected.has(pack.filename)}
				onclick={() => {
					toggleSelect(pack.filename);
				}}
				onkeydown={() => {
					toggleSelect(pack.filename);
				}}
				role="button"
				tabindex="0"
			>
				<div class="pack-icon">
					{#if pack.icon}
						<img src={pack.icon} alt={pack.name} />
					{:else}
						<Cubic />
					{/if}
				</div>
				<div class="pack-info">
					<span class="pack-name" title={pack.name}>{pack.name}</span>
					<p class="pack-description" title={pack.description}>
						{pack.description ||
							t("instanceView.mods.noDescription")}
					</p>
				</div>
			</div>
		{/each}

		{#if packs.length === 0 && !isLoading}
			<div class="empty-state">
				{t(i18nPrefix + ".empty")}
			</div>
		{/if}
	</div>
</div>

<style>
	.resources-section {
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}

	.section-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
	}

	.section-header-left {
		display: flex;
		align-items: center;
		gap: 10px;
	}

	.delete-selected-btn {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		padding: 6px 12px;
		background: rgba(255, 68, 68, 0.1);
		border: 1px solid rgba(255, 68, 68, 0.25);
		color: #ff4444;
		border-radius: var(--border-radius-sm);
		cursor: pointer;
		font-size: 0.78rem;
		font-weight: 700;
		transition: all 0.2s;
	}

	.delete-selected-btn:hover {
		background: rgba(255, 68, 68, 0.2);
		border-color: rgba(255, 68, 68, 0.4);
	}

	.rp-subtabs {
		display: flex;
		gap: 2px;
		background: rgba(255, 255, 255, 0.04);
		border-radius: var(--border-radius-sm);
		padding: 2px;
		align-self: flex-start;
	}

	.rp-subtab {
		padding: 4px 14px;
		background: transparent;
		border: none;
		color: var(--text-secondary);
		cursor: pointer;
		font-size: 0.72rem;
		font-weight: 600;
		border-radius: calc(var(--border-radius-sm) - 1px);
		transition: all 0.15s;
		white-space: nowrap;
	}

	.rp-subtab:hover {
		color: var(--text-primary);
		background: rgba(255, 255, 255, 0.04);
	}

	.rp-subtab.active {
		background: var(--accent);
		color: var(--bg-main);
	}

	.section-title {
		font-size: 1.2rem;
		font-weight: 600;
		color: var(--text-primary);
	}

	.section-actions {
		display: flex;
		gap: 8px;
	}

	.browse-btn,
	.add-btn {
		padding: 0.5rem 1rem;
		background: var(--accent);
		color: var(--bg-main);
		border: none;
		border-radius: var(--border-radius-sm);
		cursor: pointer;
		font-weight: 600;
		transition: opacity 0.2s;
	}

	.browse-btn:hover,
	.add-btn:hover {
		opacity: 0.9;
	}

	.packs-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(250px, 1fr));
		gap: 0.75rem;
	}

	.pack-card {
		/* Seria bueno agregar blur customizable. */
		background: rgba(255, 255, 255, 0.03);
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		padding: 14px;
		display: flex;
		gap: 10px;
		align-items: flex-start;
		transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
		position: relative;
		cursor: pointer;
	}

	.pack-card:hover {
		background: rgba(255, 255, 255, 0.06);
		border-color: rgba(255, 255, 255, 0.1);
		box-shadow: 0 4px 12px rgba(0, 0, 0, 0.2);
	}

	.pack-card.selected {
		border-color: var(--accent);
		background: var(--bg-card-gradient);
	}

	.pack-icon {
		width: 48px;
		height: 48px;
		background: rgba(255, 255, 255, 0.03);
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		display: flex;
		align-items: center;
		justify-content: center;
		flex-shrink: 0;
		overflow: hidden;
	}

	.pack-icon img {
		width: 100%;
		height: 100%;
		object-fit: contain;
		image-rendering: pixelated;
	}

	.pack-info {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	.pack-name {
		font-size: 0.88rem;
		font-weight: 700;
		color: var(--text-primary);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.pack-description {
		font-size: 0.72rem;
		color: var(--text-secondary);
		display: -webkit-box;
		-webkit-line-clamp: 2;
		line-clamp: 2;
		-webkit-box-orient: vertical;
		overflow: hidden;
		line-height: 1.4;
		margin-top: 4px;
	}

	.empty-state {
		grid-column: 1 / -1;
		text-align: center;
		padding: 2rem;
		color: var(--text-secondary);
		background: rgba(255, 255, 255, 0.02);
		border-radius: var(--border-radius-sm);
		border: 1px dashed rgba(255, 255, 255, 0.1);
	}
</style>
