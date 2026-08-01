<script lang="ts">
	import { t } from "$lib/i18n";
	import {
		searchModrinth,
		getModrinthProjectVersions,
		getModrinthProject,
		downloadMrpack,
		installMrpackWithUpstream,
		openUrl,
	} from "$lib/api/cubicApi";
	import type {
		ModrinthProject,
		ModrinthProjectFull,
		ModrinthVersion,
	} from "$lib/types/types";
	import { renderMarkdown } from "$lib/util/markdown";
	import Select from "$lib/components/layout/Select.svelte";
	import Icon from "$lib/icons/Icon.svelte";
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

	let needsCustomName = $state(false);
	let customName = $state("");
	let customNameError = $state<string | null>(null);

	let fullProject = $state<ModrinthProjectFull | null>(null);
	let loadingFullProject = $state(false);

	const readmeHtml = $derived(
		fullProject?.body ? renderMarkdown(fullProject.body) : "",
	);

	const versionOptions = $derived(
		versions.map((v) => ({
			value: v.id,
			label:
				v.game_versions.length > 0
					? `${v.version_number} (${v.game_versions[0]})`
					: v.version_number,
		})),
	);

	let sentinelEl: HTMLDivElement | undefined = $state();
	let initialized = $state(false);

	async function doSearch(reset?: boolean) {
		if (!reset && (searching || loadingMore)) return;

		if (reset) {
			searching = true;
		} else {
			loadingMore = true;
		}
		installError = null;
		if (reset) {
			offset = 0;
			results = [];
			fullProject = null;
		}
		try {
			const result = await searchModrinth(
				query,
				"",
				undefined,
				null,
				"downloads",
				10,
				reset ? 0 : offset,
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
			loadingMore = false;
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
		loadingFullProject = true;
		versions = [];
		fullProject = null;
		try {
			const [fetchedVersions, projectFull] = await Promise.all([
				getModrinthProjectVersions(pack.project_id),
				getModrinthProject(pack.project_id),
			]);
			versions = fetchedVersions;
			if (versions.length > 0) {
				selectedVersion = versions[0].id;
			}
			fullProject = projectFull;
		} finally {
			loadingVersions = false;
			loadingFullProject = false;
		}
	}

	function goBack() {
		selectedPack = null;
		versions = [];
		selectedVersion = "";
		fullProject = null;
	}

	async function install() {
		if (!selectedPack || !selectedVersion) return;

		const rawName = selectedPack.title;
		if (!isValidInstanceName(rawName)) {
			customName = sanitizeInstanceName(rawName);
			customNameError = null;
			needsCustomName = true;
			installError = null;
			return;
		}

		await doInstall(rawName);
	}

	async function doInstall(name: string) {
		if (!selectedPack || !selectedVersion) return;
		installing = true;
		installError = null;
		needsCustomName = false;
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
			const result = await installMrpackWithUpstream(
				mrpackPath,
				name,
				selectedPack.project_id,
				selectedVersion,
				selectedPack.icon_url ?? undefined,
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

	function confirmCustomName() {
		const trimmed = customName.trim();
		if (!trimmed) {
			customNameError = t("createInstance.emptyNameErr");
			return;
		}
		if (!isValidInstanceName(trimmed)) {
			customNameError = t("createInstance.nameInvalidChars");
			return;
		}
		doInstall(trimmed);
	}

	function cancelCustomName() {
		needsCustomName = false;
		customName = "";
		customNameError = null;
	}

	function formatDownloads(n: number): string {
		if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
		if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
		return String(n);
	}

	const FORBIDDEN_CHARS = ["/", "\\", "<", ">", ":", '"', "|", "?", "*"];
	const MAX_NAME_LEN = 16;

	function isValidInstanceName(name: string): boolean {
		const trimmed = name.trim();
		if (!trimmed) return false;
		if (trimmed.length > MAX_NAME_LEN) return false;
		if (!/^[\0-\x7F]*$/.test(trimmed)) return false;
		if (trimmed.includes("..")) return false;
		if (trimmed.split("").some((c) => FORBIDDEN_CHARS.includes(c)))
			return false;
		return true;
	}

	function sanitizeInstanceName(name: string): string {
		let clean = name
			.normalize("NFD")
			.replace(/[\u0300-\u036f]/g, "")
			.replace(/[^\x20-\x7E]/g, "")
			.replace(/[\\/<>:"|?*]/g, "")
			.replace(/\.\./g, "")
			.replace(/\s+/g, " ")
			.trim();
		if (!clean) clean = "modpack";
		if (clean.length > MAX_NAME_LEN) clean = clean.slice(0, MAX_NAME_LEN);
		return clean;
	}

	$effect(() => {
		if (!initialized) {
			initialized = true;
			doSearch(true);
		}
	});

	$effect(() => {
		const el = sentinelEl;
		if (!el) return;

		const observer = new IntersectionObserver(
			(entries) => {
				if (entries[0].isIntersecting) {
					doSearch(false);
				}
			},
			{ rootMargin: "300px" },
		);

		observer.observe(el);
		return () => observer.disconnect();
	});
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

	{#if selectedPack}
		<div class="detail-view">
			<button type="button" class="back-btn" onclick={goBack}>
				<Icon src="/images/icons/ui/chevron-left.svg" size={16} />
				Volver
			</button>

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

			<div class="detail-actions">
				<div class="version-select">
					<span class="version-label"
						>{t("createInstance.versionLabel")}</span
					>
					<Select
						bind:value={selectedVersion}
						options={versionOptions}
						placeholder={t("createInstance.selectLoaderVersion")}
						loading={loadingVersions}
						disabled={versions.length === 0 || installing}
					/>
				</div>

				{#if needsCustomName}
					<button
						type="button"
						class="btn-secondary install-btn"
						onclick={cancelCustomName}
						disabled={installing}
					>
						{t("createInstance.cancel")}
					</button>
				{:else}
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
				{/if}
			</div>

			{#if needsCustomName}
				<div class="custom-name-section">
					<p class="custom-name-hint">
						{t("createInstance.customNameNeeded")}
					</p>
					<div class="custom-name-input-row">
						<input
							type="text"
							class="text-input"
							class:error={customNameError}
							bind:value={customName}
							maxlength={16}
							disabled={installing}
							oninput={() => (customNameError = null)}
							onkeydown={(e) =>
								e.key === "Enter" && confirmCustomName()}
							placeholder={t(
								"createInstance.customNamePlaceholder",
							)}
						/>
						<button
							type="button"
							class="btn-primary"
							onclick={confirmCustomName}
							disabled={installing || !customName.trim()}
						>
							{installing
								? t("createInstance.installingModpack")
								: t("createInstance.installBtn")}
						</button>
					</div>
					{#if customNameError}
						<span class="input-error">{customNameError}</span>
					{/if}
				</div>
			{/if}

			{#if loadingFullProject}
				<div class="readme-loading">
					<Loading />
				</div>
			{:else if readmeHtml}
				<div class="detail-readme">
					<span class="readme-label">README</span>
					<div
						class="readme-content markdown-body"
						role="presentation"
						onclick={(e) => {
							const a = (e.target as HTMLElement).closest("a");
							if (a?.href && !a.href.startsWith("#")) {
								e.preventDefault();
								openUrl(a.href);
							}
						}}
					>
						{@html readmeHtml}
					</div>
				</div>
			{/if}
		</div>
	{:else}
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
					<div bind:this={sentinelEl} class="load-sentinel">
						{#if loadingMore}
							<Loading />
						{:else}
							<span class="sentinel-hint">Scroll for more</span>
						{/if}
					</div>
				{/if}
			{/if}
		</div>
	{/if}
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

	/* ── Results list ───────────────────────────── */
	.results-panel {
		flex: 1;
		overflow-y: auto;
		min-height: 0;
		max-height: 440px;
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

	.load-sentinel {
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 12px;
	}

	.sentinel-hint {
		font-size: 0.7rem;
		color: var(--text-tertiary);
	}

	/* ── Detail view ────────────────────────────── */
	.detail-view {
		flex: 1;
		overflow-y: auto;
		max-height: 440px;
		display: flex;
		flex-direction: column;
		gap: 14px;
		animation: slideIn 0.2s ease-out;
	}

	@keyframes slideIn {
		from {
			opacity: 0.5;
			transform: translateX(24px);
		}
		to {
			opacity: 1;
			transform: translateX(0);
		}
	}

	.back-btn {
		display: flex;
		align-items: center;
		gap: 6px;
		padding: 6px 10px;
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		background: transparent;
		color: var(--text-secondary);
		font-size: 0.75rem;
		font-family: inherit;
		cursor: pointer;
		align-self: flex-start;
		transition:
			color 0.15s,
			border-color 0.15s;
	}

	.back-btn:hover {
		color: var(--text-primary);
		border-color: var(--text-secondary);
	}

	.detail-header {
		display: flex;
		align-items: center;
		gap: 12px;
	}

	.detail-icon {
		width: 48px;
		height: 48px;
		border-radius: var(--border-radius-sm);
		object-fit: cover;
	}

	.detail-title-group h3 {
		margin: 0;
		font-size: 1rem;
		font-weight: 700;
	}

	.detail-author {
		font-size: 0.75rem;
		color: var(--text-secondary);
	}

	.detail-desc {
		font-size: 0.78rem;
		color: var(--text-secondary);
		line-height: 1.4;
		margin: 0;
	}

	.detail-actions {
		display: flex;
		gap: 12px;
		align-items: flex-end;
	}

	.version-select {
		display: flex;
		flex-direction: column;
		gap: 4px;
		flex: 1;
	}

	.version-label {
		font-size: 0.7rem;
		font-weight: 600;
		color: var(--text-secondary);
		text-transform: uppercase;
		letter-spacing: 0.5px;
	}

	.install-btn {
		flex-shrink: 0;
		height: fit-content;
		padding: 8px 20px;
		justify-content: center;
	}

	.detail-actions :global(.select-trigger) {
		padding: 8px 14px;
		font-size: 0.8rem;
	}

	.readme-loading {
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 30px;
	}

	.detail-readme {
		border-top: 1px solid var(--border);
		padding-top: 12px;
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.readme-label {
		font-size: 0.7rem;
		font-weight: 600;
		color: var(--text-secondary);
		text-transform: uppercase;
		letter-spacing: 0.5px;
	}

	.readme-content {
		font-size: 0.78rem;
		line-height: 1.5;
		color: var(--text-primary);
	}

	:global(.markdown-body h1),
	:global(.markdown-body h2),
	:global(.markdown-body h3),
	:global(.markdown-body h4) {
		margin: 14px 0 6px;
		font-weight: 700;
		line-height: 1.3;
	}

	:global(.markdown-body h1) {
		font-size: 1.05rem;
	}
	:global(.markdown-body h2) {
		font-size: 0.95rem;
	}
	:global(.markdown-body h3) {
		font-size: 0.85rem;
	}

	:global(.markdown-body p) {
		margin: 8px 0;
	}

	:global(.markdown-body a) {
		color: var(--accent);
		text-decoration: none;
	}

	:global(.markdown-body a:hover) {
		text-decoration: underline;
	}

	:global(.markdown-body code) {
		font-size: 0.72rem;
		padding: 1px 4px;
		border-radius: 3px;
		background: rgba(var(--accent-rgb), 0.08);
		font-family: monospace;
	}

	:global(.markdown-body pre) {
		padding: 10px;
		border-radius: var(--border-radius-sm);
		background: rgba(var(--accent-rgb), 0.04);
		border: 1px solid var(--border);
		overflow-x: auto;
		font-size: 0.72rem;
	}

	:global(.markdown-body pre code) {
		background: none;
		padding: 0;
	}

	:global(.markdown-body ul),
	:global(.markdown-body ol) {
		padding-left: 20px;
		margin: 8px 0;
	}

	:global(.markdown-body img) {
		max-width: 100%;
		border-radius: var(--border-radius-sm);
	}

	:global(.markdown-body blockquote) {
		border-left: 3px solid var(--accent);
		padding-left: 10px;
		margin: 8px 0;
		color: var(--text-secondary);
	}

	:global(.markdown-body hr) {
		border: none;
		border-top: 1px solid var(--border);
		margin: 14px 0;
	}

	.custom-name-section {
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		padding: 12px;
		background: rgba(var(--accent-rgb, 255, 255, 255), 0.03);
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.custom-name-hint {
		font-size: 0.78rem;
		color: var(--text-secondary);
		margin: 0;
		line-height: 1.4;
	}

	.custom-name-input-row {
		display: flex;
		gap: 8px;
		align-items: center;
	}

	.custom-name-input-row :global(.text-input) {
		flex: 1;
		padding: 8px 12px;
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		background: var(--bg-input);
		color: var(--text-primary);
		font-size: 0.82rem;
		font-family: inherit;
		outline: none;
	}

	.custom-name-input-row :global(.text-input:focus) {
		border-color: var(--accent);
	}

	.custom-name-input-row :global(.text-input.error) {
		border-color: var(--color-error) !important;
		box-shadow: 0 0 0 1px var(--color-error) !important;
	}

	.input-error {
		font-size: 0.7rem;
		color: var(--color-error);
		display: block;
	}
</style>
