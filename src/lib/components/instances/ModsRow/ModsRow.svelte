<script lang="ts">
	import {
		getInstanceMods,
		toggleInstanceMod,
		deleteInstanceFile,
	} from "$lib/api/cubicApi";
	import { type ModDto } from "$lib/types/types";
	import { t } from "$lib/i18n";
	import ModalBase from "../../layout/ModalBase.svelte";
	import { SvelteSet } from "svelte/reactivity";
	import Trash from "$lib/icons/Trash.svelte";
	import Lupa from "$lib/icons/Lupa.svelte";
	import ModCard from "./ModCard.svelte";

	let { instanceId } = $props<{ instanceId: string }>();
	let mods = $state<ModDto[]>([]);
	let selected = new SvelteSet<string>();
	let searchQuery = $state("");
	let prevInstanceId = $state<string>("");
	let loading = $state(true);
	let bulkDeleteModal = $state(false);

	let filteredMods = $derived(
		mods.filter(
			(mod) =>
				mod.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
				(mod.description &&
					mod.description
						.toLowerCase()
						.includes(searchQuery.toLowerCase())) ||
				(mod.authors &&
					mod.authors.some((a) =>
						a.toLowerCase().includes(searchQuery.toLowerCase()),
					)),
		),
	);

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
		} else {
			selected.add(filename);
		}
	}

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
		<div class="search-bar">
			<Lupa width="20" height="20" />
			<input
				type="text"
				disabled={mods.length === 0}
				placeholder={t("instanceView.mods.searchPlaceholder")}
				bind:value={searchQuery}
			/>
		</div>
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
			{#each filteredMods as mod (mod.filename)}
				<ModCard
					{mod}
					isSelected={selected.has(mod.filename)}
					onToggleSelect={toggleSelect}
					onToggleEnable={handleToggle}
				/>
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
	.search-bar {
		flex: 0 0 250px;
		position: relative;
		display: flex;
		align-items: center;
	}

	.search-bar :global(svg) {
		position: absolute;
		left: 10px;
		pointer-events: none;
		color: var(--text-secondary);
		opacity: 0.5;
		z-index: 1;
	}

	.search-bar input {
		width: 100%;
		padding: 8px 12px 8px 36px;
		background: rgba(255, 255, 255, 0.05);
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		color: var(--text-primary);
		font-size: 0.85rem;
		outline: none;
		transition: all 0.2s;
	}

	.search-bar input:focus {
		border-color: var(--accent);
		background: rgba(255, 255, 255, 0.08);
	}

	.search-bar input::placeholder {
		color: var(--text-secondary);
		opacity: 0.6;
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
