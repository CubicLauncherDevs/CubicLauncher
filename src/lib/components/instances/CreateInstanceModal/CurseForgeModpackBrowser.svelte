<script lang="ts">
	import { t } from "$lib/i18n";
	import {
		searchCurseForgeModpacks,
		getCurseForgeProjectFiles,
		getCurseForgeProjectDescription,
		getCurseForgeFileDownloadUrl,
		downloadCurseForgeModpack,
		installCurseForgeModpack,
		openUrl,
	} from "$lib/api/cubicApi";
	import type { CurseForgeProject, CurseForgeFile } from "$lib/types/types";
	import HtmlRenderer from "$lib/components/ui/HtmlRenderer.svelte";
	import Loading from "$lib/icons/Loading.svelte";
	import {
		isValidInstanceName,
		sanitizeInstanceName,
	} from "$lib/utils/instanceName";
	import ModpackBrowser, { type ModpackItem } from "./ModpackBrowser.svelte";

	let {
		onInstalled,
	}: {
		onInstalled?: () => void;
	} = $props();

	const limit = 10;

	let query = $state("");
	let projects = $state<CurseForgeProject[]>([]);
	let items = $state<ModpackItem[]>([]);
	let totalHits = $state(0);
	let searching = $state(false);
	let loadingMore = $state(false);
	let offset = $state(0);
	let selectedItem = $state<ModpackItem | null>(null);
	let selectedPack = $state<CurseForgeProject | null>(null);
	let versions = $state<CurseForgeFile[]>([]);
	let selectedVersion = $state<string>("");
	let loadingVersions = $state(false);
	let installing = $state(false);
	let installError = $state<string | null>(null);
	let installStep = $state<string>("");
	let needsCustomName = $state(false);
	let customName = $state("");
	let customNameError = $state<string | null>(null);

	let description = $state<string>("");
	let loadingDescription = $state(false);

	let searchGen = 0;

	const versionOptions = $derived(
		versions.map((v) => ({
			value: String(v.id),
			label:
				v.gameVersions.length > 0
					? `${v.fileName} (${v.gameVersions[0]})`
					: v.fileName,
		})),
	);

	function projectToItem(pack: CurseForgeProject): ModpackItem {
		return {
			id: pack.id,
			title: pack.name,
			description: pack.summary,
			iconUrl: pack.logo?.url ?? null,
			downloads: Number(pack.downloadCount),
			author: pack.authors.map((a) => a.name).join(", ") || undefined,
		};
	}

	async function doSearch(reset?: boolean) {
		if (!reset && (searching || loadingMore)) return;

		const gen = ++searchGen;

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
			description = "";
		}
		try {
			const result = await searchCurseForgeModpacks(
				query,
				"",
				undefined,
				"downloads",
				limit,
				reset ? 0 : offset,
			);
			if (gen !== searchGen) return;
			if (result) {
				projects = reset ? result.data : [...projects, ...result.data];
				items = projects.map(projectToItem);
				totalHits = Number(result.pagination.totalCount);
				offset = reset ? result.pagination.pageSize : offset + result.pagination.pageSize;
			}
		} finally {
			if (gen === searchGen) {
				searching = false;
				loadingMore = false;
			}
		}
	}

	function handleSearch() {
		selectedItem = null;
		selectedPack = null;
		versions = [];
		selectedVersion = "";
		description = "";
		doSearch(true);
	}

	function handleLoadMore() {
		doSearch(false);
	}

	async function handleSelect(item: ModpackItem) {
		selectedItem = item;
		selectedVersion = "";
		loadingVersions = true;
		loadingDescription = true;
		versions = [];
		description = "";
		selectedPack = projects.find((p) => p.id === item.id) ?? null;

		if (!selectedPack) {
			loadingVersions = false;
			loadingDescription = false;
			return;
		}

		try {
			const [fetchedVersions, fetchedDescription] = await Promise.all([
				getCurseForgeProjectFiles(
					selectedPack.id,
					"",
					undefined,
				),
				getCurseForgeProjectDescription(selectedPack.id),
			]);
			versions = fetchedVersions;
			if (versions.length > 0) {
				selectedVersion = String(versions[0].id);
			}
			description = fetchedDescription ?? "";
		} finally {
			loadingVersions = false;
			loadingDescription = false;
		}
	}

	function handleBack() {
		selectedItem = null;
		selectedPack = null;
		versions = [];
		selectedVersion = "";
		description = "";
	}

	async function handleInstall() {
		if (!selectedPack || !selectedVersion) return;

		const rawName = selectedPack.name;
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
			const file = versions.find((v) => String(v.id) === selectedVersion);
			if (!file) throw new Error("Version not found");

			let url = file.downloadUrl;
			if (!url) {
				url = await getCurseForgeFileDownloadUrl(
					selectedPack.id,
					file.id,
					file.fileName,
				);
			}
			if (!url) throw new Error("No download URL found for this modpack");

			installStep = t("createInstance.downloadingModpack");
			const packPath = await downloadCurseForgeModpack(url, file.id);
			if (!packPath) throw new Error("Failed to download modpack");

			installStep = t("createInstance.importingBtn");
			const result = await installCurseForgeModpack(
				packPath,
				name,
				selectedPack.id,
				file.id,
				selectedPack.logo?.url ?? undefined,
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
	searchPlaceholder={t("createInstance.curseforgeModpackSearchPlaceholder")}
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
	{#snippet detailExtra(item)}
		{#if item}
			<div class="detail-links">
				<button
					type="button"
					class="link-btn"
					onclick={() =>
						openUrl(
							`https://www.curseforge.com/minecraft/modpacks/${selectedPack?.slug ?? ""}`,
						)}
				>
					{t("createInstance.viewOnCurseForge")}
				</button>
			</div>

			{#if loadingDescription}
				<div class="readme-loading">
					<Loading />
				</div>
			{:else if description}
				<div class="detail-readme">
					<span class="readme-label">README</span>
					<div class="readme-content">
						<HtmlRenderer
							source={description}
							class="markdown-body"
							onLinkClick={openUrl}
						/>
					</div>
				</div>
			{/if}
		{/if}
	{/snippet}
</ModpackBrowser>

<style>
	.detail-links {
		border-top: 1px solid var(--border);
		padding-top: 12px;
	}

	.link-btn {
		background: transparent;
		border: none;
		color: var(--accent);
		cursor: pointer;
		font-size: 0.78rem;
		padding: 0;
	}

	.link-btn:hover {
		text-decoration: underline;
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
</style>
