<script lang="ts">
import {
	fetchFtbPackIds,
	fetchFtbModpackDetails,
	installFtbModpack,
} from "$lib/api/cubicApi";
import type { FTBModpack, FTBModpackVersion } from "$lib/types/types";

let {
	onInstalled,
}: {
	onInstalled?: () => void;
} = $props();

let view = $state<"browse" | "versions">("browse");
let searchQuery = $state("");
let results = $state<FTBModpack[]>([]);
let loading = $state(false);
let selectedPack = $state<FTBModpack | null>(null);
let selectedVersion = $state<FTBModpackVersion | null>(null);
let installing = $state(false);
let error = $state<string | null>(null);
let packIds = $state<number[]>([]);
let detailOffset = $state(0);
const BATCH_SIZE = 5;
let loadingMore = $state(false);
let debounceTimer: ReturnType<typeof setTimeout> | undefined;

import { onMount } from "svelte";

onMount(() => {
	loadPacks("");
});

async function loadPacks(query: string) {
	loading = true;
	results = [];
	detailOffset = 0;
	packIds = await fetchFtbPackIds(query, 50);
	await loadMore();
	loading = false;
}

async function loadMore() {
	if (loadingMore || detailOffset >= packIds.length) return;
	loadingMore = true;
	const batch = packIds.slice(detailOffset, detailOffset + BATCH_SIZE);
	if (batch.length === 0) { loadingMore = false; return; }
	const details = await fetchFtbModpackDetails(batch);
	results = [...results, ...details];
	detailOffset += BATCH_SIZE;
	loadingMore = false;
}

function onSearchInput() {
	clearTimeout(debounceTimer);
	debounceTimer = setTimeout(() => {
		loadPacks(searchQuery);
	}, 300);
}

function selectPack(pack: FTBModpack) {
	selectedPack = pack;
	view = "versions";
	selectedVersion = null;
}

function selectVersion(version: FTBModpackVersion) {
	selectedVersion = version;
}

function goBack() {
	view = "browse";
	selectedVersion = null;
}

async function install() {
	if (!selectedPack || !selectedVersion) return;
	installing = true;
	error = null;

	try {
		await installFtbModpack(
			selectedPack.id,
			selectedVersion.id,
			selectedPack.name,
			selectedPack.name,
			selectedVersion.name,
			selectedVersion.minecraft,
			selectedVersion.loader || null,
			selectedVersion.loaderVersion || null,
			() => {
				reset();
				onInstalled?.();
			},
			(err: unknown) => {
				error = `Error al instalar: ${err}`;
			},
		);
	} finally {
		installing = false;
	}
}

function reset() {
	view = "browse";
	searchQuery = "";
	results = [];
	loading = false;
	selectedPack = null;
	selectedVersion = null;
	installing = false;
	error = null;
	packIds = [];
	detailOffset = 0;
	loadingMore = false;
}
</script>

<div class="ftb-browser">
	<div class="ftb-search">
		<input
			type="text"
			class="text-input"
			placeholder="Buscar modpacks..."
			bind:value={searchQuery}
			oninput={onSearchInput}
		/>
	</div>

	{#if view === "browse"}
		{#if loading && results.length === 0}
			<div class="ftb-loading">
				<p>Cargando modpacks...</p>
			</div>
		{:else}
			<div class="ftb-pack-list">
				{#each results as pack (pack.id)}
					<button
						type="button"
						class="ftb-pack-card"
						class:selected={selectedPack?.id === pack.id}
						onclick={() => selectPack(pack)}
					>
						{#if pack.icon}
							<img src={pack.icon} alt="" class="ftb-pack-icon" />
						{/if}
						<div class="ftb-pack-info">
							<span class="ftb-pack-name">{pack.name}</span>
							<span class="ftb-pack-meta">{pack.authors.join(", ")} · {pack.installs} instalaciones</span>
						</div>
					</button>
				{/each}
			</div>
			{#if detailOffset < packIds.length}
				<button
					type="button"
					class="ftb-load-more"
					onclick={loadMore}
					disabled={loadingMore}
				>
					{loadingMore ? "Cargando..." : "Cargar más"}
				</button>
			{/if}
		{/if}
	{:else if view === "versions" && selectedPack}
		<div class="ftb-versions-header">
			<button type="button" class="ftb-back-btn" onclick={goBack}>← Volver</button>
			<span class="ftb-versions-title">{selectedPack.name}</span>
		</div>
		<div class="ftb-version-list">
			{#each selectedPack.versions as ver (ver.id)}
				<button
					type="button"
					class="ftb-version-card"
					class:selected={selectedVersion?.id === ver.id}
					onclick={() => selectVersion(ver)}
				>
					<div class="ftb-version-info">
						<span class="ftb-version-name">{ver.name}</span>
						<span class="ftb-version-meta">{ver.type} · {ver.minecraft} · {ver.loader || "Vanilla"}</span>
					</div>
				</button>
			{/each}
		</div>

		{#if selectedVersion}
			<div class="ftb-version-detail">
				<div class="ftb-detail-row">
					<span class="info-label">Versión</span>
					<span class="info-value">{selectedVersion.name}</span>
				</div>
				<div class="ftb-detail-row">
					<span class="info-label">Tipo</span>
					<span class="info-value">{selectedVersion.type}</span>
				</div>
				<div class="ftb-detail-row">
					<span class="info-label">Minecraft</span>
					<span class="info-value">{selectedVersion.minecraft}</span>
				</div>
				<div class="ftb-detail-row">
					<span class="info-label">Loader</span>
					<span class="info-value">{selectedVersion.loader || "Vanilla"}{selectedVersion.loaderVersion ? " " + selectedVersion.loaderVersion : ""}</span>
				</div>
			</div>

			{#if error}
				<div class="ftb-error">{error}</div>
			{/if}

			<button
				type="button"
				class="btn-primary ftb-install-btn"
				onclick={install}
				disabled={installing}
			>
				{installing ? "Instalando..." : "Instalar Modpack"}
			</button>
		{/if}
	{/if}
</div>

<style>
	.ftb-browser {
		margin-top: 12px;
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		background: rgba(0, 0, 0, 0.15);
		max-height: 380px;
		overflow: hidden;
		display: flex;
		flex-direction: column;
	}

	.ftb-search {
		padding: 8px;
		border-bottom: 1px solid var(--border);
	}

	.ftb-search :global(input) {
		width: 100%;
		box-sizing: border-box;
	}

	.ftb-loading {
		padding: 32px 16px;
		text-align: center;
		color: var(--text-secondary);
		font-size: 0.9rem;
	}

	.ftb-pack-list {
		flex: 1;
		overflow-y: auto;
		padding: 4px;
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	.ftb-pack-card {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 8px;
		background: none;
		border: 1px solid transparent;
		border-radius: 6px;
		color: var(--text-primary);
		cursor: pointer;
		text-align: left;
		width: 100%;
		font-size: 0.8rem;
		transition: background 0.1s;
	}

	.ftb-pack-card:hover {
		background: rgba(255, 255, 255, 0.04);
	}

	.ftb-pack-card.selected {
		background: rgba(var(--accent-rgb), 0.08);
		border-color: var(--accent);
	}

	.ftb-pack-icon {
		width: 32px;
		height: 32px;
		border-radius: 6px;
		object-fit: cover;
		flex-shrink: 0;
	}

	.ftb-pack-info {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	.ftb-pack-name {
		font-weight: 600;
		font-size: 0.8rem;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.ftb-pack-meta {
		font-size: 0.7rem;
		color: var(--text-secondary);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.ftb-load-more {
		width: 100%;
		padding: 10px;
		background: none;
		border: 1px dashed var(--border);
		border-radius: 6px;
		color: var(--text-secondary);
		font-size: 0.8rem;
		cursor: pointer;
		transition: background 0.1s, color 0.1s;
		margin-top: 4px;
	}

	.ftb-load-more:hover:not(:disabled) {
		background: rgba(255, 255, 255, 0.03);
		color: var(--text-primary);
	}

	.ftb-load-more:disabled {
		cursor: default;
		opacity: 0.5;
	}

	.ftb-versions-header {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 8px;
		border-bottom: 1px solid var(--border);
	}

	.ftb-back-btn {
		background: none;
		border: none;
		color: var(--text-secondary);
		cursor: pointer;
		font-size: 0.8rem;
		padding: 2px 6px;
		border-radius: 4px;
	}

	.ftb-back-btn:hover {
		color: var(--text-primary);
		background: rgba(255, 255, 255, 0.05);
	}

	.ftb-versions-title {
		font-weight: 600;
		font-size: 0.85rem;
		flex: 1;
	}

	.ftb-version-list {
		flex: 1;
		overflow-y: auto;
		padding: 4px;
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	.ftb-version-card {
		display: flex;
		align-items: center;
		padding: 8px;
		background: none;
		border: 1px solid transparent;
		border-radius: 6px;
		color: var(--text-primary);
		cursor: pointer;
		text-align: left;
		width: 100%;
		font-size: 0.8rem;
		transition: background 0.1s;
	}

	.ftb-version-card:hover {
		background: rgba(255, 255, 255, 0.04);
	}

	.ftb-version-card.selected {
		background: rgba(var(--accent-rgb), 0.08);
		border-color: var(--accent);
	}

	.ftb-version-info {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	.ftb-version-name {
		font-weight: 600;
		font-size: 0.8rem;
	}

	.ftb-version-meta {
		font-size: 0.7rem;
		color: var(--text-secondary);
	}

	.ftb-version-detail {
		padding: 10px;
		border-top: 1px solid var(--border);
		display: flex;
		flex-direction: column;
		gap: 6px;
		background: rgba(255, 255, 255, 0.02);
	}

	.ftb-detail-row {
		display: flex;
		align-items: baseline;
		gap: 8px;
	}

	.ftb-detail-row .info-label {
		font-size: 0.75rem;
		font-weight: 600;
		color: var(--text-secondary);
		text-transform: uppercase;
		letter-spacing: 0.5px;
		min-width: 70px;
		flex-shrink: 0;
	}

	.ftb-detail-row .info-value {
		font-size: 0.85rem;
		color: var(--text-primary);
	}

	.ftb-error {
		color: var(--color-error);
		font-size: 0.8rem;
		background: rgba(var(--color-error-rgb), 0.1);
		border: 1px solid rgba(var(--color-error-rgb), 0.2);
		border-radius: 6px;
		padding: 10px;
		text-align: center;
		font-weight: 500;
		margin: 8px;
	}

	.ftb-install-btn {
		margin: 8px;
	}
</style>
