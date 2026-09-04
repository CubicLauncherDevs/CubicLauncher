<script lang="ts">
	import { t } from "$lib/i18n";
	import { showError, showSuccess } from "$lib/state/state.svelte";
	import {
		syncSkinCloset,
		removeSkinFromCloset,
		renameSkinInCloset,
		equipSkinFromCloset,
	} from "$lib/api/cubicApi";
	import type { SkinClosetEntry } from "$lib/types/types";
	import SkinClosetItem from "./SkinClosetItem.svelte";

	interface Props {
		uuid: string;
		activeSkinId: string | null;
		processing: boolean;
		refreshTrigger?: number;
		onEquipped?: (entry: SkinClosetEntry) => void;
	}

	let {
		uuid,
		activeSkinId,
		processing,
		refreshTrigger = 0,
		onEquipped,
	}: Props = $props();

	let entries = $state<SkinClosetEntry[]>([]);
	let loading = $state(false);
	// Flag no reactivo para evitar que Svelte rastree `loading` dentro de los
	// $effect y genere rerenders/reintentos infinitos.
	let fetching = false;

	async function loadCloset(silent = false) {
		if (fetching) return;
		fetching = true;
		if (!silent) loading = true;
		try {
			entries = await syncSkinCloset(uuid);
		} catch (err) {
			showError(t("errors.title"), String(err));
		} finally {
			fetching = false;
			if (!silent) loading = false;
		}
	}

	async function handleEquip(entry: SkinClosetEntry) {
		try {
			await equipSkinFromCloset(uuid, entry.id);
			showSuccess(
				t("userMenu.skinCape.skinCloset.equippedTitle"),
				t("userMenu.skinCape.skinCloset.equippedDesc"),
			);
			onEquipped?.(entry);
		} catch (err) {
			showError(t("errors.title"), String(err));
		}
	}

	async function handleRemove(entryId: string) {
		try {
			await removeSkinFromCloset(uuid, entryId);
			showSuccess(
				t("userMenu.skinCape.skinCloset.removedTitle"),
				t("userMenu.skinCape.skinCloset.removedDesc"),
			);
			await loadCloset(true);
		} catch (err) {
			showError(t("errors.title"), String(err));
		}
	}

	async function handleRename(entryId: string, alias: string) {
		try {
			await renameSkinInCloset(uuid, entryId, alias);
			showSuccess(
				t("userMenu.skinCape.skinCloset.renamedTitle"),
				t("userMenu.skinCape.skinCloset.renamedDesc"),
			);
			await loadCloset(true);
		} catch (err) {
			showError(t("errors.title"), String(err));
		}
	}

	$effect(() => {
		void loadCloset();
	});

	$effect(() => {
		// eslint-disable-next-line @typescript-eslint/no-unused-expressions
		refreshTrigger;
		void loadCloset(true);
	});
</script>

<div class="closet-section">
	<h5 class="subsection-title">
		{t("userMenu.skinCape.skinCloset.title")}
		{#if entries.length > 0}
			<span class="closet-count">{entries.length}</span>
		{/if}
	</h5>

	{#if loading && entries.length === 0}
		<div class="closet-loading">
			<span class="spinner"></span>
		</div>
	{:else if entries.length === 0}
		<p class="closet-empty">
			{t("userMenu.skinCape.skinCloset.noSavedSkins")}
		</p>
	{:else}
		<div class="closet-grid" role="list">
			{#each entries as entry (entry.id)}
				<div role="listitem">
					<SkinClosetItem
						{entry}
						isActive={entry.id === activeSkinId}
						processing={processing || loading}
						onEquip={() => handleEquip(entry)}
						onRemove={() => handleRemove(entry.id)}
						onRename={(alias) => handleRename(entry.id, alias)}
					/>
				</div>
			{/each}
		</div>
	{/if}
</div>

<style>
	.closet-section {
		display: flex;
		flex-direction: column;
		gap: 14px;
		background: var(--bg-card);
		border: 1px solid var(--border);
		border-radius: var(--border-radius);
		padding: 14px;
	}

	.subsection-title {
		margin: 0;
		font-size: 0.75rem;
		font-weight: 700;
		color: var(--text-secondary);
		text-transform: uppercase;
		letter-spacing: 0.6px;
		display: flex;
		align-items: center;
		gap: 10px;
	}

	.closet-count {
		background: var(--accent);
		color: var(--accent-text);
		padding: 2px 8px;
		border-radius: 999px;
		font-size: 0.65rem;
	}

	.closet-loading {
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 24px;
	}

	.spinner {
		width: 18px;
		height: 18px;
		border: 2px solid var(--border);
		border-top-color: var(--accent);
		border-radius: 50%;
		animation: spin 0.8s linear infinite;
	}

	@keyframes spin {
		to {
			transform: rotate(360deg);
		}
	}

	.closet-empty {
		margin: 0;
		font-size: 0.8rem;
		color: var(--text-muted);
		padding: 12px 0;
	}

	.closet-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(128px, 1fr));
		gap: 12px;
	}

	@media (max-width: 520px) {
		.closet-grid {
			grid-template-columns: repeat(auto-fill, minmax(100px, 1fr));
			gap: 10px;
		}
	}
</style>
