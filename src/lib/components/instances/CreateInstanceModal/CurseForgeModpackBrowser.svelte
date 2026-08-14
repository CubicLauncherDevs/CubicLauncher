<script lang="ts">
	import { t } from "$lib/i18n";
	import {
		searchCurseForgeModpacks,
		getCurseForgeProjectFiles,
		downloadCurseForgeFile,
		installCurseForgeModpack,
	} from "$lib/api/cubicApi";
	import type { CurseForgeProject, CurseForgeFile } from "$lib/types/types";
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
	let files = $state<CurseForgeFile[]>([]);
	let selectedVersionId = $state<string>("");
	let loadingVersions = $state(false);
	let installing = $state(false);
	let installError = $state<string | null>(null);
	let installStep = $state<string>("");
	let needsCustomName = $state(false);
	let customName = $state("");
	let customNameError = $state<string | null>(null);

	let searchGen = 0;

	const selectedFile = $derived(
		files.find((f) => String(f.id) === selectedVersionId) ?? null,
	);

	const versionOptions = $derived(
		files.map((f) => ({
			value: String(f.id),
			label:
				f.gameVersions.length > 0
					? `${f.gameVersions[0]} · ${f.fileName}`
					: f.fileName,
		})),
	);

	function projectToItem(pack: CurseForgeProject): ModpackItem {
		return {
			id: pack.id,
			title: pack.name,
			description: pack.summary,
			iconUrl: pack.logo?.url,
			downloads: pack.downloadCount,
			author: pack.authors[0]?.name,
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
		}
		try {
			const result = await searchCurseForgeModpacks(
				query,
				"popularity",
				limit,
				reset ? 0 : offset,
			);
			if (gen !== searchGen) return;
			if (result) {
				projects = reset ? result.data : [...projects, ...result.data];
				items = projects.map(projectToItem);
				totalHits = result.pagination.totalCount;
				offset = reset ? limit : offset + limit;
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
		files = [];
		selectedVersionId = "";
		doSearch(true);
	}

	function handleLoadMore() {
		doSearch(false);
	}

	async function handleSelect(item: ModpackItem) {
		selectedItem = item;
		selectedVersionId = "";
		loadingVersions = true;
		files = [];
		selectedPack = projects.find((p) => p.id === item.id) ?? null;

		try {
			const fetchedFiles = await getCurseForgeProjectFiles(
				item.id as number,
			);
			files = fetchedFiles.filter((f) => f.isAvailable);
			if (files.length > 0) {
				selectedVersionId = String(files[0].id);
			}
		} finally {
			loadingVersions = false;
		}
	}

	function handleBack() {
		selectedItem = null;
		selectedPack = null;
		files = [];
		selectedVersionId = "";
	}

	async function handleInstall() {
		if (!selectedPack || !selectedFile) return;

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
		if (!selectedPack || !selectedFile) return;
		installing = true;
		installError = null;
		needsCustomName = false;
		try {
			installStep = t("createInstance.downloadingModpack");
			const archivePath = await downloadCurseForgeFile(
				selectedFile.modId,
				selectedFile.id,
			);
			if (!archivePath) throw new Error("Failed to download modpack");

			installStep = t("createInstance.importingBtn");
			const iconUrl = selectedPack.logo?.url;
			const result = await installCurseForgeModpack(
				archivePath,
				name.trim(),
				String(selectedPack.id),
				String(selectedFile.id),
				iconUrl,
			);
			if (result) {
				onInstalled?.();
			} else {
				installError = "Error al importar el modpack";
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
	bind:selectedVersion={selectedVersionId}
	{loadingVersions}
	{installing}
	{installError}
	{installStep}
	{needsCustomName}
	bind:customName
	{customNameError}
	searchPlaceholder={t("createInstance.curseForgeSearchPlaceholder")}
	emptySearchingText={t("createInstance.searchingModpacks")}
	emptyNoResultsText={t("createInstance.noModpacksFound")}
	onSearch={handleSearch}
	onLoadMore={handleLoadMore}
	onSelect={handleSelect}
	onBack={handleBack}
	onInstall={handleInstall}
	onConfirmCustomName={handleConfirmCustomName}
	onCancelCustomName={handleCancelCustomName}
/>
