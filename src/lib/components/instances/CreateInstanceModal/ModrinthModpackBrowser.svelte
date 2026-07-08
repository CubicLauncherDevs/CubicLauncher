<script lang="ts">
	import { t } from "$lib/i18n";
	import {
		searchModrinth,
		getModrinthProjectVersions,
		downloadMrpack,
		installMrpackWithUpstream,
	} from "$lib/api/cubicApi";
	import type { ModrinthProject, ModrinthVersion } from "$lib/types/types";
	import Loading from "$lib/icons/Loading.svelte";

	let {
		onInstalled,
	}: {
		onInstalled?: () => void;
	} = $props();

	let query = $state("");
	let results = $state<ModrinthProject[]>([]);
	let totalHits = $state(0);
	let searching = $state(false);
	let loadingMore = $state(false);
	let offset = $state(0);
	let selectedPack = $state<ModrinthProject | null>(null);
	let versions = $state<ModrinthVersion[]>([]);
	let selectedVersion = $state<string>("");
	let loadingVersions = $state(false);
	let installing = $state(false);
	let installError = $state<string | null>(null);
	let installStep = $state<string>("");

	async function doSearch(reset?: boolean) {
		if (!query.trim()) return;
		searching = true;
		installError = null;
		if (reset) {
			offset = 0;
			results = [];
		}
		try {
			const result = await searchModrinth(
				query,
				"",
				undefined,
				null,
				"downloads",
				24,
				reset ? 0 : offset,
				undefined,
				"modpack",
			);
			if (result) {
				if (reset) {
					results = result.hits;
				} else {
					results = [...results, ...result.hits];
				}
				totalHits = result.total_hits;
				offset = reset ? result.limit : offset + result.limit;
			}
		} finally {
			searching = false;
		}
	}

	function handleSearch() {
		selectedPack = null;
		versions = [];
		selectedVersion = "";
		doSearch(true);
	}

	async function selectPack(pack: ModrinthProject) {
		selectedPack = pack;
		selectedVersion = "";
		loadingVersions = true;
		versions = [];
		try {
			versions = await getModrinthProjectVersions(pack.project_id);
			if (versions.length > 0) {
				selectedVersion = versions[0].id;
			}
		} finally {
			loadingVersions = false;
		}
	}

	async function install() {
		if (!selectedPack || !selectedVersion) return;
		installing = true;
		installError = null;
		try {
			const ver = versions.find((v) => v.id === selectedVersion);
			if (!ver) throw new Error("Version not found");
			const primaryFile =
				ver.files.find((f) => f.primary) ?? ver.files[0];
			if (!primaryFile) throw new Error("No file found in version");

			installStep = t("createInstance.downloadingModpack");
			const mrpackPath = await downloadMrpack(
				primaryFile.url,
				selectedVersion,
			);
			if (!mrpackPath) throw new Error("Failed to download modpack");

			installStep = t("createInstance.importingBtn");
			const name = selectedPack.title;
			const result = await installMrpackWithUpstream(
				mrpackPath,
				name,
				selectedPack.project_id,
				selectedVersion,
				() => {
					onInstalled?.();
				},
				(err) => {
					installError = String(err);
				},
			);
			if (!result && !installError) {
				installError = "Failed to install modpack";
			}
		} catch (e) {
			installError = String(e);
		} finally {
			installing = false;
			installStep = "";
		}
	}

	function formatDownloads(n: number): string {
		if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
		if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
		return String(n);
	}
</script>

<div class="modpack-browser">
	<div class="search-bar">
		<input
			type="text"
			class="search-input"
			bind:value={query}
			placeholder={t("createInstance.modpackSearchPlaceholder")}
			onkeydown={(e) => e.key === "Enter" && handleSearch()}
			disabled={searching}
		/>
		<button
			type="button"
			class="btn-primary search-btn"
			onclick={handleSearch}
			disabled={searching || !query.trim()}
		>
			{#if searching}
				<Loading />
			{/if}
			{t("createInstance.searchBtn")}
		</button>
	</div>

	{#if installError}
		<div class="error-msg">{installError}</div>
	{/if}

	{#if installing}
		<div class="installing-overlay">
			<Loading />
			<span>{installStep}</span>
		</div>
	{/if}

	<div class="browser-layout">
		<div class="results-panel">
			{#if searching && results.length === 0}
				<div class="empty-state">
					<Loading />
					<span>{t("createInstance.searchingModpacks")}</span>
				</div>
			{:else if results.length === 0}
				<div class="empty-state">
					{t("createInstance.noModpacksFound")}
				</div>
			{:else}
				<div class="results-grid">
					{#each results as pack (pack.project_id)}
						<button
							type="button"
							class="pack-card"
							class:selected={selectedPack?.project_id ===
								pack.project_id}
							onclick={() => selectPack(pack)}
						>
							<div class="pack-icon-wrap">
								{#if pack.icon_url}
									<img
										src={pack.icon_url}
										alt=""
										class="pack-icon"
									/>
								{/if}
							</div>
							<div class="pack-info">
								<span class="pack-title">{pack.title}</span>
								<span class="pack-desc">{pack.description}</span
								>
								<span class="pack-meta">
									{formatDownloads(pack.downloads)}
									{t("createInstance.downloads")}
								</span>
							</div>
						</button>
					{/each}
				</div>
				{#if results.length < totalHits}
					<button
						type="button"
						class="btn-secondary load-more"
						onclick={() => doSearch()}
						disabled={loadingMore}
					>
						{t("createInstance.loadMore")}
					</button>
				{/if}
			{/if}
		</div>

		{#if selectedPack}
			<div class="detail-panel">
				<div class="detail-header">
					{#if selectedPack.icon_url}
						<img
							src={selectedPack.icon_url}
							alt=""
							class="detail-icon"
						/>
					{/if}
					<div class="detail-title-group">
						<h3>{selectedPack.title}</h3>
						<span class="detail-author">{selectedPack.author}</span>
					</div>
				</div>
				<p class="detail-desc">{selectedPack.description}</p>

				<div class="version-select">
					<span class="version-label"
						>{t("createInstance.versionLabel")}</span
					>
					<select
						bind:value={selectedVersion}
						disabled={loadingVersions || versions.length === 0}
					>
						{#each versions as v (v)}
							<option value={v.id}>
								{v.version_number}
								{#if v.game_versions.length > 0}
									({v.game_versions[0]})
								{/if}
							</option>
						{/each}
					</select>
				</div>

				<button
					type="button"
					class="btn-primary install-btn"
					onclick={install}
					disabled={installing || !selectedVersion}
				>
					{installing
						? t("createInstance.installingModpack")
						: t("createInstance.installBtn")}
				</button>
			</div>
		{/if}
	</div>
</div>

<style>
	.modpack-browser {
		display: flex;
		flex-direction: column;
		gap: 12px;
		height: 100%;
		min-height: 300px;
	}

	.search-bar {
		display: flex;
		gap: 8px;
		align-items: center;
	}

	.search-input {
		flex: 1;
		padding: 8px 12px;
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		background: var(--bg-input);
		color: var(--text-primary);
		font-size: 0.82rem;
		outline: none;
	}

	.search-input:focus {
		border-color: var(--accent);
	}

	.search-btn {
		flex-shrink: 0;
		display: flex;
		align-items: center;
		gap: 6px;
	}

	.error-msg {
		color: var(--color-error);
		font-size: 0.8rem;
		padding: 8px 12px;
		background: rgba(var(--color-error-rgb), 0.1);
		border-radius: var(--border-radius-sm);
	}

	.installing-overlay {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 10px;
		padding: 16px;
		color: var(--text-secondary);
		font-size: 0.85rem;
	}

	.browser-layout {
		display: flex;
		gap: 16px;
		flex: 1;
		min-height: 0;
	}

	.results-panel {
		flex: 1;
		overflow-y: auto;
		min-width: 0;
	}

	.empty-state {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 10px;
		padding: 40px 16px;
		color: var(--text-muted);
		font-size: 0.82rem;
	}

	.results-grid {
		display: flex;
		flex-direction: column;
		gap: 6px;
	}

	.pack-card {
		display: flex;
		gap: 10px;
		padding: 10px;
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		background: transparent;
		color: inherit;
		cursor: pointer;
		text-align: left;
		width: 100%;
		transition:
			background 0.15s ease,
			border-color 0.15s ease;
	}

	.pack-card:hover {
		background: var(--bg-item-active);
	}

	.pack-card.selected {
		border-color: var(--accent);
		background: rgba(var(--accent-rgb), 0.08);
	}

	.pack-icon-wrap {
		width: 48px;
		height: 48px;
		border-radius: var(--border-radius-sm);
		overflow: hidden;
		flex-shrink: 0;
		background: var(--bg-card);
	}

	.pack-icon {
		width: 100%;
		height: 100%;
		object-fit: cover;
	}

	.pack-info {
		display: flex;
		flex-direction: column;
		gap: 3px;
		min-width: 0;
		flex: 1;
	}

	.pack-title {
		font-size: 0.82rem;
		font-weight: 600;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.pack-desc {
		font-size: 0.72rem;
		color: var(--text-secondary);
		display: -webkit-box;
		-webkit-line-clamp: 2;
		-webkit-box-orient: vertical;
		line-clamp: 2;
		overflow: hidden;
	}

	.pack-meta {
		font-size: 0.65rem;
		color: var(--text-tertiary);
	}

	.detail-panel {
		width: 280px;
		flex-shrink: 0;
		display: flex;
		flex-direction: column;
		gap: 12px;
		padding: 14px;
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		background: var(--bg-card);
	}

	.detail-header {
		display: flex;
		align-items: center;
		gap: 10px;
	}

	.detail-icon {
		width: 40px;
		height: 40px;
		border-radius: var(--border-radius-sm);
		object-fit: cover;
	}

	.detail-title-group h3 {
		margin: 0;
		font-size: 0.9rem;
		font-weight: 700;
	}

	.detail-author {
		font-size: 0.72rem;
		color: var(--text-secondary);
	}

	.detail-desc {
		font-size: 0.75rem;
		color: var(--text-secondary);
		line-height: 1.4;
		margin: 0;
		display: -webkit-box;
		-webkit-line-clamp: 4;
		-webkit-box-orient: vertical;
		line-clamp: 4;
		overflow: hidden;
	}

	.version-select {
		display: flex;
		flex-direction: column;
		gap: 4px;
	}

	.version-label {
		font-size: 0.7rem;
		font-weight: 600;
		color: var(--text-secondary);
		text-transform: uppercase;
		letter-spacing: 0.5px;
	}

	.version-select select {
		padding: 6px 8px;
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		background: var(--bg-input);
		color: var(--text-primary);
		font-size: 0.78rem;
	}

	.install-btn {
		margin-top: auto;
		width: 100%;
		justify-content: center;
	}

	.load-more {
		width: 100%;
		justify-content: center;
		margin-top: 8px;
	}
</style>
