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
	import Icon from "$lib/icons/Icon.svelte";
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

	function portal(el: HTMLElement) {
		document.body.appendChild(el);
		return {
			destroy() {
				el.remove();
			},
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

	const chipTitle = $derived(
		pack
			? `${pack.name} · ${pack.version} · ${t("modpack.installationVersion")}: ${pack.game_version} · ${pack.source}`
			: "",
	);
	const lockLabel = $derived(
		pack ? t(pack.locked ? "modpack.unlock" : "modpack.lock") : "",
	);
</script>

{#if loading || error || pack}
	<div
		class="pack-summary"
		aria-label={t("modpack.title")}
		aria-busy={loading || changing}
	>
		{#if loading}
			<span class="pack-spinner" aria-hidden="true"></span>
		{/if}
		{#if error}
			<span class="pack-error" role="alert" title={error}>{error}</span>
			<button
				type="button"
				class="chip-btn warn"
				title={t("modpack.retry")}
				aria-label={t("modpack.retry")}
				disabled={loading || changing}
				onclick={() => load(id)}
			>
				<Icon name="ui:refresh" size={14} />
			</button>
		{/if}
		{#if pack}
			<span class="pack-chip" title={chipTitle}>
				<Icon name="instance:puzzle" size={13} />
				<strong class="pack-name">{pack.name}</strong>
				<span class="lock-dot" class:locked={pack.locked}></span>
			</span>
			<button
				type="button"
				class="chip-btn"
				title={lockLabel}
				aria-label={lockLabel}
				disabled={busy || changing || loading}
				onclick={toggleLock}
			>
				<Icon name={pack.locked ? "ui:unlock" : "ui:lock"} size={14} />
			</button>
			<button
				type="button"
				class="chip-btn"
				title={t("modpack.update")}
				aria-label={t("modpack.update")}
				disabled={busy || changing || loading}
				onclick={() => (updateOpen = true)}
			>
				<Icon name="ui:refresh" size={14} />
			</button>
			{#if pack.needs_inventory}
				<button
					type="button"
					class="chip-btn warn"
					title={t("modpack.needsInventory")}
					aria-label={t("modpack.restoreInventory")}
					disabled={busy || changing || loading}
					onclick={restoreInventory}
				>
					<Icon name="ui:download" size={14} />
				</button>
			{/if}
		{/if}
	</div>
{/if}
{#if updateOpen && pack}
	<div use:portal>
		<ModpackUpdateModal
			bind:open={updateOpen}
			{id}
			{pack}
			{busy}
			onupdated={() => load(id)}
		/>
	</div>
{/if}

<style>
	.pack-summary {
		display: flex;
		align-items: center;
		gap: 6px;
		min-width: 0;
	}
	.pack-chip {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		min-width: 0;
		max-width: 180px;
		padding: 3px 8px;
		border: 1px solid var(--border);
		border-radius: 5px;
		background: var(--bg-card);
		color: var(--text-secondary);
		font-size: 0.7rem;
		font-weight: 600;
	}
	.pack-name {
		min-width: 0;
		color: var(--text-primary);
		font-weight: 600;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.lock-dot {
		width: 6px;
		height: 6px;
		flex-shrink: 0;
		border-radius: 50%;
		background: var(--text-tertiary);
	}
	.lock-dot.locked {
		background: var(--accent);
	}
	.pack-error {
		min-width: 0;
		max-width: 220px;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		color: var(--color-error);
		font-size: 0.7rem;
	}
	.chip-btn {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: 26px;
		height: 26px;
		flex-shrink: 0;
		padding: 0;
		border: 1px solid var(--border);
		border-radius: 6px;
		background: transparent;
		color: var(--text-tertiary);
		cursor: pointer;
		transition:
			background 0.15s,
			color 0.15s,
			border-color 0.15s;
	}
	.chip-btn:hover:not(:disabled) {
		background: var(--bg-card);
		color: var(--text-primary);
		border-color: var(--text-tertiary);
	}
	.chip-btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}
	.chip-btn.warn {
		color: var(--color-error);
		border-color: rgba(var(--color-error-rgb), 0.35);
	}
	.pack-spinner {
		width: 14px;
		height: 14px;
		flex-shrink: 0;
		border-radius: 50%;
		border: 2px solid var(--border);
		border-top-color: var(--accent);
		animation: pack-spin 0.8s linear infinite;
	}
	@keyframes pack-spin {
		to {
			transform: rotate(360deg);
		}
	}
</style>
