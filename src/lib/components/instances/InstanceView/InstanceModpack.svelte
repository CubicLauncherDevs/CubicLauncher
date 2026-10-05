<script lang="ts">
	import { onDestroy } from "svelte";
	import {
		getInstanceModpack,
		setModpackLocked,
		restoreModpackInventory,
		type PackState,
	} from "$lib/api/modpackApi";
	import {
		refreshInstanceMods,
		registerModsRefreshCallback,
	} from "$lib/api/launcherService";
	import { t } from "$lib/i18n";
	import ModpackUpdateModal from "./ModpackUpdateModal.svelte";

	let { id, busy = false }: { id: string; busy?: boolean } = $props();
	let pack = $state<Omit<PackState, "files"> | null>(null);
	let loading = $state(true);
	let changing = $state(false);
	let error = $state("");
	let updateOpen = $state(false);
	let reloadPending = $state(false);
	let generation = 0;
	let disposed = false;
	onDestroy(() => {
		disposed = true;
		generation++;
	});

	function metadata(state: PackState | null) {
		if (!state) return null;
		return {
			schema_version: state.schema_version,
			source: state.source,
			project_id: state.project_id,
			version_id: state.version_id,
			name: state.name,
			version: state.version,
			game_version: state.game_version,
			locked: state.locked,
			needs_inventory: state.needs_inventory,
		};
	}

	async function load(instanceId: string) {
		const request = ++generation;
		loading = true;
		error = "";
		try {
			const result = await getInstanceModpack(instanceId);
			if (!disposed && request === generation) pack = metadata(result);
		} catch (e) {
			if (!disposed && request === generation) error = String(e);
		} finally {
			if (!disposed && request === generation) loading = false;
		}
	}
	$effect(() => {
		const instanceId = id;
		pack = null;
		updateOpen = false;
		void load(instanceId);
		return registerModsRefreshCallback(instanceId, () => {
			if (changing || updateOpen) reloadPending = true;
			else void load(instanceId);
		});
	});
	$effect(() => {
		if (reloadPending && !changing && !updateOpen) {
			reloadPending = false;
			void load(id);
		}
	});

	async function toggleLock() {
		if (!pack || busy || changing) return;
		const instanceId = id;
		const request = generation;
		changing = true;
		error = "";
		try {
			const result = await setModpackLocked(instanceId, !pack.locked);
			refreshInstanceMods(instanceId);
			if (!disposed && request === generation) pack = metadata(result);
		} catch (e) {
			if (!disposed && request === generation) error = String(e);
		} finally {
			if (!disposed && request === generation) changing = false;
		}
	}

	async function restoreInventory() {
		if (!pack?.needs_inventory || busy || changing || loading || disposed)
			return;
		const instanceId = id;
		const request = ++generation;
		changing = true;
		error = "";
		try {
			const result = await restoreModpackInventory(instanceId);
			refreshInstanceMods(instanceId);
			if (!disposed && request === generation) pack = metadata(result);
		} catch (e) {
			if (!disposed && request === generation) error = String(e);
		} finally {
			if (!disposed && request === generation) changing = false;
		}
	}
</script>

{#if loading || error || pack}
	<section
		class="pack-summary"
		aria-label={t("modpack.title")}
		aria-busy={loading || changing}
	>
		{#if loading}
			<span role="status">{t("createInstance.loading")}</span>
		{/if}
		{#if error}
			<p role="alert">{error}</p>
			<button
				type="button"
				class="btn-secondary"
				disabled={loading || changing}
				onclick={() => load(id)}>{t("modpack.retry")}</button
			>
		{/if}
		{#if pack}
			<div class="identity">
				<strong>{pack.name}</strong>
				<span
					>{pack.version} · {t("modpack.installationVersion")}: {pack.game_version}
					· {pack.source}</span
				>
			</div>
			<span>{t(pack.locked ? "modpack.locked" : "modpack.unlocked")}</span
			>
			<button
				type="button"
				class="btn-secondary"
				disabled={busy || changing || loading}
				onclick={toggleLock}
				>{t(pack.locked ? "modpack.unlock" : "modpack.lock")}</button
			>
			<button
				type="button"
				class="btn-primary"
				disabled={busy || changing || loading}
				onclick={() => (updateOpen = true)}
				>{t("modpack.update")}</button
			>
			{#if pack.needs_inventory}
				<div class="inventory-notice">
					<span role="status">{t("modpack.needsInventory")}</span>
					<button
						type="button"
						class="btn-secondary"
						disabled={busy || changing || loading}
						onclick={restoreInventory}
						>{t("modpack.restoreInventory")}</button
					>
				</div>
			{/if}
		{/if}
	</section>
{/if}
{#if updateOpen && pack}
	<ModpackUpdateModal
		bind:open={updateOpen}
		{id}
		{pack}
		{busy}
		onupdated={() => load(id)}
	/>
{/if}

<style>
	.pack-summary {
		display: flex;
		align-items: center;
		flex-wrap: wrap;
		gap: 12px;
		padding: 12px 24px;
		border-bottom: 1px solid var(--border);
		color: var(--text-secondary);
		font-size: 0.8rem;
	}
	.identity {
		display: flex;
		flex-direction: column;
		gap: 4px;
		flex: 1;
		min-width: 160px;
		overflow-wrap: anywhere;
	}
	.inventory-notice {
		display: flex;
		align-items: center;
		flex-wrap: wrap;
		gap: 12px;
		flex-basis: 100%;
	}
	strong {
		color: var(--text-primary);
	}
	p {
		color: var(--color-error);
	}
</style>
