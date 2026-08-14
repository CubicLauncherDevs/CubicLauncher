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
	import type { ModrinthProject, ModrinthVersion } from "$lib/types/types";
	import { renderMarkdown } from "$lib/util/markdown";
	import {
		isValidInstanceName,
		sanitizeInstanceName,
	} from "$lib/utils/instanceName";
	import Loading from "$lib/icons/Loading.svelte";
	import ModpackBrowser, { type ModpackItem } from "./ModpackBrowser.svelte";

	let {
		onInstalled,
	}: {
		onInstalled?: () => void;
	} = $props();

	const limit = 10;

	let query = $state("");
	let projects = $state<ModrinthProject[]>([]);
	let items = $state<ModpackItem[]>([]);
	let totalHits = $state(0);
	let searching = $state(false);
	let loadingMore = $state(false);
	let offset = $state(0);
	let selectedItem = $state<ModpackItem | null>(null);
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

	let fullProject = $state<string>("");
	let loadingFullProject = $state(false);

	const versionOptions = $derived(
		versions.map((v) => ({
			value: v.id,
			label:
				v.game_versions.length > 0
					? `${v.version_number} (${v.game_versions[0]})`
					: v.version_number,
		})),
	);

	const readmeHtml = $derived(fullProject ? renderMarkdown(fullProject) : "");

	function projectToItem(pack: ModrinthProject): ModpackItem {
		return {
			id: pack.project_id,
			title: pack.title,
			description: pack.description,
			iconUrl: pack.icon_url,
			downloads: pack.downloads,
			author: pack.author,
		};
	}

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
			projects = [];
			items = [];
			fullProject = "";
		}
		try {
			const result = await searchModrinth(
				query,
				"",
				undefined,
				null,
				"downloads",
				limit,
				reset ? 0 : offset,
				"modpack",
			);
			if (result) {
				projects = reset ? result.hits : [...projects, ...result.hits];
				items = projects.map(projectToItem);
				totalHits = result.total_hits;
				offset = reset ? result.limit : offset + result.limit;
			}
		} finally {
			searching = false;
			loadingMore = false;
		}
	}

	function handleSearch() {
		selectedItem = null;
		selectedPack = null;
		versions = [];
		selectedVersion = "";
		doSearch(true);
	}

	function handleLoadMore() {
		doSearch(false);
	}

	async function handleSelect(item: ModpackItem) {
		selectedItem = item;
		selectedVersion = "";
		loadingVersions = true;
		loadingFullProject = true;
		versions = [];
		fullProject = "";
		selectedPack = projects.find((p) => p.project_id === item.id) ?? null;

		if (!selectedPack) {
			loadingVersions = false;
			loadingFullProject = false;
			return;
		}

		try {
			const [fetchedVersions, projectFull] = await Promise.all([
				getModrinthProjectVersions(selectedPack.project_id),
				getModrinthProject(selectedPack.project_id),
			]);
			versions = fetchedVersions;
			if (versions.length > 0) {
				selectedVersion = versions[0].id;
			}
			fullProject = projectFull?.body ?? "";
		} finally {
			loadingVersions = false;
			loadingFullProject = false;
		}
	}

	function handleBack() {
		selectedItem = null;
		selectedPack = null;
		versions = [];
		selectedVersion = "";
		fullProject = "";
	}

	async function handleInstall() {
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

	function handleConfirmCustomName() {
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

	function handleCancelCustomName() {
		needsCustomName = false;
		customName = "";
		customNameError = null;
	}

	$effect(() => {
		doSearch(true);
	});
</script>

<ModpackBrowser
	bind:query
	{items}
	{totalHits}
	{searching}
	{loadingMore}
	bind:selectedItem
	{versionOptions}
	bind:selectedVersion
	{loadingVersions}
	{installing}
	{installError}
	{installStep}
	{needsCustomName}
	bind:customName
	{customNameError}
	searchPlaceholder={t("createInstance.modpackSearchPlaceholder")}
	emptySearchingText={t("createInstance.searchingModpacks")}
	emptyNoResultsText={t("createInstance.noModpacksFound")}
	onSearch={handleSearch}
	onLoadMore={handleLoadMore}
	onSelect={handleSelect}
	onBack={handleBack}
	onInstall={handleInstall}
	onConfirmCustomName={handleConfirmCustomName}
	onCancelCustomName={handleCancelCustomName}
>
	{#snippet detailExtra(_item)}
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
	{/snippet}
</ModpackBrowser>

<style>
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
</style>
