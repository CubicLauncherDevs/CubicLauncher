<script lang="ts">
	import {
		getInstanceMods,
		toggleInstanceMod,
		deleteInstanceFile,
	} from "$lib/api/cubicApi";
	import { type ModDto } from "$lib/types/types";
	import { t } from "$lib/i18n";
	import ModalBase from "../layout/ModalBase.svelte";
	import { SvelteSet } from "svelte/reactivity";
	import Trash from "$lib/icons/Trash.svelte";

	let { instanceId } = $props<{ instanceId: string }>();
	let mods = $state<ModDto[]>([]);
	// Ignorar, svelte que jode con esos falsos positivos
	// dios mioooo
	let selected = new SvelteSet<string>();
	let prevInstanceId = $state<string>("");
	let loading = $state(true);
	let bulkDeleteModal = $state(false);
	$effect(() => {
		if (instanceId && instanceId !== prevInstanceId) {
			prevInstanceId = instanceId;
			loading = true;
			getInstanceMods(instanceId)
				.then((data) => {
					mods = data;
				})
				.finally(() => {
					loading = false;
				});
		}
	});

	function toggleSelect(filename: string) {
		if (selected.has(filename)) {
			selected.delete(filename);
			return;
		} else {
			selected.add(filename);
			return;
		}
	}

	// function toggleSelectAll() {
	// 	if (selected.size === mods.length) {
	// 		selected.clear();
	// 	} else {
	// 		selected.clear();
	// 		for (const m of mods) {
	// 			selected.add(m.filename);
	// 		}
	// 	}
	// }

	async function handleToggle(mod: ModDto) {
		const newEnabled = !mod.enabled;
		mod.enabled = newEnabled;

		await toggleInstanceMod(instanceId, mod.filename, newEnabled);

		mods = await getInstanceMods(instanceId);
	}

	async function handleBulkDelete() {
		const count = selected.size;
		if (count === 0) return;
		for (const filename of selected) {
			await deleteInstanceFile(instanceId, "mods", filename);
		}
		selected.clear();
		bulkDeleteModal = false;
		mods = await getInstanceMods(instanceId);
	}
</script>

<div class="mods-section">
	<div class="section-header">
		<span class="section-title"
			>{t("instanceView.mods.title")} ({mods.length})</span
		>
		<div class="selection-actions">
			<span class="selection-count">{selected.size}</span>
			<button
				type="button"
				class="delete-selected-btn"
				disabled={selected.size == 0}
				onclick={() => {
					bulkDeleteModal = true;
				}}
			>
				<Trash width="14" height="14" />

				{t("instanceView.deleteSelected")}
			</button>
		</div>
	</div>
	{#if loading}
		<div class="mods-loading">
			<div class="minimal-spinner"></div>
		</div>
	{:else}
		<div class="mods-grid">
			{#each mods as mod (mod.filename)}
				<div
					class="mod-card"
					class:disabled={!mod.enabled}
					class:selected={selected.has(mod.filename)}
					role="button"
					tabindex="0"
					onkeydown={() => {
						toggleSelect(mod.filename);
					}}
					onclick={() => {
						toggleSelect(mod.filename);
					}}
				>
					<div class="mod-icon">
						{#if mod.icon}
							<img src={mod.icon} alt={mod.name} />
						{:else}
							<img src="/images/cubic.svg" alt={mod.name} />
						{/if}
					</div>
					<div class="mod-info">
						<div class="mod-name-row">
							<span class="mod-name" title={mod.name}
								>{mod.name}</span
							>
							<span class="mod-version"
								>{mod.version ||
									t("instanceView.mods.jarFile")}</span
							>
						</div>
						<p class="mod-description" title={mod.description}>
							{mod.description ||
								t("instanceView.mods.noDescription")}
						</p>
						{#if mod.authors && mod.authors.length > 0}
							<span
								class="mod-authors"
								title={mod.authors.join(", ")}
							>
								{t("instanceView.mods.authors")}: {mod.authors.join(
									", ",
								)}
							</span>
						{/if}
					</div>
					<div class="mod-actions">
						<div class="mod-status-toggle">
							<input
								type="checkbox"
								checked={mod.enabled}
								onchange={() => handleToggle(mod)}
							/>
						</div>
					</div>
				</div>
			{/each}
			{#if mods.length === 0}
				<div class="empty-mods">
					{t("instanceView.mods.empty")}
				</div>
			{/if}
		</div>
	{/if}
</div>
<ModalBase bind:open={bulkDeleteModal} title={t("sidebar.modals.deleteTitle")}>
	<p
		style="font-size: 0.9rem; color: var(--text-secondary); line-height: 1.4;"
	>
		{t("instanceView.mods.bulkDelete")}
	</p>
	{#snippet footer()}
		<button
			type="button"
			class="btn-secondary"
			onclick={() => (bulkDeleteModal = false)}
			>{t("sidebar.modals.cancel")}</button
		>
		<button
			type="button"
			class="btn-primary"
			style="background: var(--color-error); color: white;"
			onclick={handleBulkDelete}>{t("sidebar.modals.deleteBtn")}</button
		>
	{/snippet}
</ModalBase>

<style>
	.mods-section {
		margin-bottom: 24px;
	}

	.section-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 12px;
	}

	.section-title {
		font-size: 1.2rem;
		font-weight: 600;
		color: var(--text-primary);
	}

	.selection-actions {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.selection-count {
		font-size: 0.75rem;
		font-weight: 700;
		color: var(--accent);
		background: rgba(255, 255, 255, 0.05);
		border: 1px solid var(--border);
		padding: 1px 8px;
		border-radius: var(--border-radius-sm);
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

	.delete-selected-btn:disabled {
		background: rgba(255, 255, 255, 0.05);
		border-color: rgba(255, 255, 255, 0.1);
		color: rgba(255, 255, 255, 0.35);
		cursor: not-allowed;
		opacity: 0.6;
	}

	.delete-selected-btn::not(:disabled):hover {
		background: rgba(255, 68, 68, 0.2);
		border-color: rgba(255, 68, 68, 0.4);
	}

	.mods-loading {
		display: flex;
		justify-content: center;
		align-items: center;
		padding: 48px 0;
	}

	.minimal-spinner {
		width: 32px;
		height: 32px;
		border: 2px solid var(--border);
		border-top-color: var(--accent);
		border-radius: 50%;
		animation: spin 0.8s linear infinite;
		will-change: transform;
	}

	@keyframes spin {
		to {
			transform: rotate(360deg);
		}
	}

	.mods-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));
		gap: 12px;
		padding: 4px;
	}

	.mod-card {
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

	.mod-card:hover {
		background: rgba(255, 255, 255, 0.06);
		border-color: rgba(255, 255, 255, 0.1);
		box-shadow: 0 4px 12px rgba(0, 0, 0, 0.2);
	}

	.mod-card.selected {
		border-color: var(--text-secondary);
		background: var(--bg-card-gradient);
	}

	.mod-card.disabled {
		opacity: 0.4;
		filter: grayscale(100%);
	}

	.mod-icon {
		width: 44px;
		height: 44px;
		background: rgba(255, 255, 255, 0.03);
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		display: flex;
		align-items: center;
		justify-content: center;
		flex-shrink: 0;
		overflow: hidden;
	}

	.mod-icon img {
		width: 100%;
		height: 100%;
		object-fit: contain;
		image-rendering: pixelated;
	}

	.mod-info {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	.mod-name-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 8px;
		margin-bottom: 2px;
	}

	.mod-name {
		font-size: 0.88rem;
		font-weight: 700;
		color: var(--text-primary);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.mod-version {
		font-size: 0.65rem;
		color: var(--text-secondary);
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 0.5px;
		background: rgba(255, 255, 255, 0.05);
		padding: 1px 6px;
		border-radius: var(--border-radius-sm);
		border: 1px solid var(--border);
	}

	.mod-description {
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

	.mod-authors {
		font-size: 0.65rem;
		color: var(--accent);
		opacity: 0.6;
		margin-top: 6px;
		font-weight: 600;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.mod-actions {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 6px;
	}

	.mod-status-toggle {
		display: flex;
		align-items: center;
	}

	.mod-status-toggle input[type="checkbox"] {
		appearance: none;
		width: 32px;
		height: 18px;
		background: rgba(255, 255, 255, 0.1);
		border-radius: 20px;
		position: relative;
		cursor: pointer;
		outline: none;
		transition: background 0.3s;
	}

	.mod-status-toggle input[type="checkbox"]::after {
		content: "";
		position: absolute;
		top: 2px;
		left: 2px;
		width: 14px;
		height: 14px;
		background: #fff;
		border-radius: 50%;
		transition: transform 0.3s;
	}

	.mod-status-toggle input[type="checkbox"]:checked {
		background: #4caf50;
	}

	.mod-status-toggle input[type="checkbox"]:checked::after {
		transform: translateX(14px);
	}

	.empty-mods {
		grid-column: 1 / -1;
		text-align: center;
		padding: 40px;
		color: var(--text-secondary);
		font-size: 0.85rem;
		background: rgba(255, 255, 255, 0.02);
		border: 1px dashed var(--border);
		border-radius: var(--border-radius-sm);
		text-transform: uppercase;
		letter-spacing: 1px;
	}

	@media (max-width: 700px) {
		.mods-grid {
			grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
		}
	}

	@media (max-width: 550px) {
		.mods-grid {
			grid-template-columns: 1fr;
			gap: 8px;
		}
	}

	@media (max-width: 400px) {
		.mods-grid {
			grid-template-columns: 1fr;
			gap: 6px;
		}
	}
</style>
