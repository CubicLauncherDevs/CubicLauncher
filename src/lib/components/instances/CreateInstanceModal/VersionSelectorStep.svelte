<script lang="ts">
	import { onMount } from "svelte";
	import { SvelteSet } from "svelte/reactivity";
	import {
		versionsState,
		loadInstalledVersions,
	} from "$lib/state/versionsState.svelte";
	import Select from "$lib/components/layout/Select.svelte";
	import Icon from "$lib/icons/Icon.svelte";
	import { t } from "$lib/i18n";
	import {
		getFabricLoaderVersions,
		getQuiltLoaderVersions,
	} from "$lib/api/cubicApi";
	import {
		createCatalogCache,
		type GroupedLoaderVersions,
	} from "$lib/components/layout/VersionDownloader/versionCatalog";
	import { getSelectionCatalog } from "./availableVersions";

	let {
		selectedLoader = $bindable<string>("vanilla"),
		selectedMcVersion = $bindable<string>(""),
		selectedLoaderVersion = $bindable<string>(""),
		compact = false,
		includeAvailable = false,
		loading = $bindable(false),
	}: {
		selectedLoader: string;
		selectedMcVersion: string;
		selectedLoaderVersion: string;
		compact?: boolean;
		includeAvailable?: boolean;
		loading?: boolean;
	} = $props();

	const catalogCache = createCatalogCache<GroupedLoaderVersions>();
	const loaderCache = createCatalogCache<string[]>();
	let catalog = $state<GroupedLoaderVersions | null>(null);
	let catalogLoader = $state("");
	let remoteLoaderVersions = $state<string[]>([]);
	let remoteLoaderKey = $state("");
	let sourceError = $state("");
	const loadingMc = $derived(
		versionsState.loading ||
			(includeAvailable && catalogLoader !== selectedLoader),
	);
	const loadingLoader = $derived(
		loadingMc ||
			(includeAvailable &&
				selectedLoader !== "vanilla" &&
				!!selectedMcVersion &&
				remoteLoaderKey !== `${selectedLoader}:${selectedMcVersion}`),
	);

	$effect(() => {
		loading = loadingMc || loadingLoader;
	});

	$effect(() => {
		if (!includeAvailable) return;
		const loader = selectedLoader;
		let cancelled = false;
		sourceError = "";
		catalogLoader = "";
		catalog = null;
		catalogCache
			.get(loader, () => getSelectionCatalog(loader))
			.then((result) => {
				if (!cancelled) catalog = result;
			})
			.catch((error) => {
				if (!cancelled) sourceError = String(error);
			})
			.finally(() => {
				if (!cancelled) catalogLoader = loader;
			});
		return () => {
			cancelled = true;
		};
	});

	$effect(() => {
		if (!includeAvailable || loadingMc) return;
		const loader = selectedLoader;
		const mc = selectedMcVersion;
		const key = `${loader}:${mc}`;
		remoteLoaderKey = "";
		remoteLoaderVersions = [];
		if (!mc || loader === "vanilla") return;
		let cancelled = false;
		const groupedVersions =
			catalog?.byGame.get(mc)?.map((v) => v.display_version) ?? [];
		loaderCache
			.get(key, async () => {
				if (loader === "fabric")
					return (await getFabricLoaderVersions(mc)).map(
						(v) => v.version,
					);
				if (loader === "quilt")
					return (await getQuiltLoaderVersions(mc)).map(
						(v) => v.version,
					);
				return groupedVersions;
			})
			.then((result) => {
				if (!cancelled) remoteLoaderVersions = result;
			})
			.catch((error) => {
				if (!cancelled) sourceError = String(error);
			})
			.finally(() => {
				if (!cancelled) remoteLoaderKey = key;
			});
		return () => {
			cancelled = true;
		};
	});

	$effect(() => {
		if (selectedLoader == null || selectedLoader === "") {
			selectedLoader = "vanilla";
		}
	});

	$effect(() => {
		if (selectedMcVersion == null) {
			selectedMcVersion = "";
		}
	});

	$effect(() => {
		if (selectedLoaderVersion == null) {
			selectedLoaderVersion = "";
		}
	});

	const LOADERS = [
		{
			value: "vanilla",
			label: "Vanilla",
			iconName: "brand:vanilla",
		},
		{
			value: "fabric",
			label: "Fabric",
			iconName: "brand:fabric",
		},
		{
			value: "forge",
			label: "Forge",
			iconName: "brand:forge",
		},
		{
			value: "neoforge",
			label: "NeoForge",
			iconName: "brand:neoforged",
		},
		{
			value: "quilt",
			label: "Quilt",
			iconName: "brand:quilt",
		},
		{ value: "optifine", label: "OptiFine", iconName: "brand:optifine" },
	];

	function compareVersions(a: string, b: string): number {
		const aParts = a.split(".").map((n) => parseInt(n, 10) || 0);
		const bParts = b.split(".").map((n) => parseInt(n, 10) || 0);
		for (let i = 0; i < Math.max(aParts.length, bParts.length); i++) {
			const av = aParts[i] ?? 0;
			const bv = bParts[i] ?? 0;
			if (av !== bv) return bv - av;
		}
		return b.localeCompare(a, undefined, { numeric: true });
	}

	function getMcVersionsForLoader(loader: string): string[] {
		const parsed = versionsState.mcVersions;
		if (!parsed) return [];

		let baseVersions: string[] = [];
		if (loader === "vanilla") {
			baseVersions = Array.from(parsed.vanilla);
		} else if (loader === "fabric") {
			baseVersions = Array.from(
				new SvelteSet([...parsed.fabric, ...parsed.vanilla]),
			);
		} else if (loader === "quilt") {
			baseVersions = Array.from(
				new SvelteSet([...parsed.quilt, ...parsed.vanilla]),
			);
		} else if (loader === "forge") {
			baseVersions = Array.from(parsed.forge).map((v) => {
				const idx = v.indexOf("-forge-");
				return idx >= 0 ? v.substring(0, idx) : v;
			});
		} else if (loader === "optifine") {
			baseVersions = Array.from(parsed.optifine).map(
				(v) => v.split("-OptiFine_")[0],
			);
		} else if (loader === "neoforge") {
			baseVersions = Array.from(parsed.neoforge).map((v) => {
				const idx = v.indexOf("-neoforge-");
				return idx >= 0 ? v.substring(0, idx) : v;
			});
		}

		return Array.from(new SvelteSet(baseVersions)).sort(compareVersions);
	}

	const availableMcVersions = $derived(
		includeAvailable
			? Array.from(
					new SvelteSet([
						...(catalogLoader === selectedLoader
							? (catalog?.gameVersions ?? [])
							: []),
						...getMcVersionsForLoader(selectedLoader),
					]),
				)
			: getMcVersionsForLoader(selectedLoader),
	);

	const mcVersionOptions = $derived(
		availableMcVersions.map((v) => ({ value: v, label: v })),
	);

	const mcPlaceholder = $derived(
		!loadingMc && availableMcVersions.length === 0
			? t(
					includeAvailable
						? "versionDownloader.notFound"
						: "createInstance.noVersionsErr",
				)
			: t("createInstance.selectMcVersion"),
	);

	const availableLoaderVersions = $derived.by(() => {
		if (!selectedMcVersion || selectedLoader === "vanilla") return [];
		const key = `${selectedLoader}:${selectedMcVersion}`;
		const installed =
			versionsState.loaderVersions?.get(key) ?? new Set<string>();
		return Array.from(
			new SvelteSet([
				...installed,
				...(includeAvailable && remoteLoaderKey === key
					? remoteLoaderVersions
					: []),
			]),
		).sort(compareVersions);
	});

	const loaderVersionOptions = $derived(
		availableLoaderVersions.map((v) => ({ value: v, label: v })),
	);

	$effect(() => {
		if (loadingMc || (includeAvailable && sourceError)) return;
		if (availableMcVersions.length === 0) {
			selectedMcVersion = "";
			return;
		}
		if (
			!selectedMcVersion ||
			!availableMcVersions.includes(selectedMcVersion)
		) {
			selectedMcVersion = availableMcVersions[0];
		}
	});

	$effect(() => {
		if (loadingLoader || (includeAvailable && sourceError)) return;
		if (selectedLoader === "vanilla") {
			selectedLoaderVersion = "";
			return;
		}
		if (availableLoaderVersions.length === 0) {
			selectedLoaderVersion = "";
			return;
		}
		if (
			!selectedLoaderVersion ||
			!availableLoaderVersions.includes(selectedLoaderVersion)
		) {
			selectedLoaderVersion = availableLoaderVersions[0];
		}
	});

	onMount(() => {
		if (!versionsState.loaded && !versionsState.loading) {
			loadInstalledVersions();
		}
	});
</script>

<div class="version-selector" class:compact aria-busy={loading}>
	<div class="loader-unified">
		{#each LOADERS as loader (loader.value)}
			<button
				type="button"
				class="loader-btn"
				class:active={selectedLoader === loader.value}
				aria-label={loader.label}
				aria-pressed={selectedLoader === loader.value}
				onclick={() => (selectedLoader = loader.value)}
			>
				<Icon name={loader.iconName} size={compact ? 16 : 20} />
				<span>{loader.label}</span>
			</button>
		{/each}
	</div>

	<div class="linked-selects">
		<Select
			bind:value={selectedMcVersion}
			options={mcVersionOptions}
			placeholder={mcPlaceholder}
			loading={loadingMc}
			loadingPlaceholder={t("createInstance.loading")}
			disabled={loadingMc || mcVersionOptions.length === 0}
		/>

		<Select
			bind:value={selectedLoaderVersion}
			options={loaderVersionOptions}
			placeholder={selectedLoader === "vanilla"
				? t("createInstance.noLoader")
				: t("createInstance.selectLoaderVersion")}
			loading={loadingLoader}
			loadingPlaceholder={t("createInstance.loading")}
			disabled={selectedLoader === "vanilla" ||
				loadingLoader ||
				loaderVersionOptions.length === 0}
		/>
	</div>
	{#if sourceError}
		<p role="alert">{sourceError}</p>
	{/if}
</div>

<style>
	.version-selector {
		display: flex;
		flex-direction: column;
		gap: 16px;
		height: 100%;
	}

	.loader-unified {
		display: flex;
		width: 100%;
	}

	.loader-btn {
		--btn-bg: rgba(var(--accent-rgb, 255, 255, 255), 0.03);
		flex: 1;
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 8px;
		padding: 12px 8px;
		background: var(--btn-bg);
		border: 1px solid var(--border);
		color: var(--text-secondary);
		font-family: inherit;
		font-size: 0.8rem;
		font-weight: 600;
		cursor: pointer;
		position: relative;
		margin-left: -1px;
		z-index: 0;
		transition:
			background-color 0.15s,
			color 0.15s,
			border-color 0.15s,
			box-shadow 0.15s;
	}

	.loader-btn:first-child {
		margin-left: 0;
		border-radius: var(--border-radius-sm) 0 0 var(--border-radius-sm);
	}

	.loader-btn:last-child {
		border-radius: 0 var(--border-radius-sm) var(--border-radius-sm) 0;
	}

	.loader-btn:hover {
		background: rgba(var(--accent-rgb, 255, 255, 255), 0.06);
		color: var(--text-primary);
		z-index: 1;
	}

	.loader-btn.active {
		background: rgba(var(--accent-rgb, 255, 255, 255), 0.1);
		border-color: var(--accent);
		color: var(--text-primary);
		z-index: 2;
	}

	.loader-btn :global(.icon-svg) {
		width: 20px;
		height: 20px;
	}

	.loader-btn span {
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	@media (max-width: 500px) {
		.loader-btn span {
			display: none;
		}
	}

	.linked-selects {
		display: flex;
		width: 100%;
	}

	.linked-selects > :global(.custom-select-container) {
		flex: 1;
		min-width: 0;
	}

	/* Left select: right border and radii removed */
	.linked-selects
		> :global(.custom-select-container:first-child .select-trigger) {
		border-top-right-radius: 0;
		border-bottom-right-radius: 0;
		border-right: none;
	}

	/* Right select: left radii removed */
	.linked-selects
		> :global(.custom-select-container:last-child .select-trigger) {
		border-top-left-radius: 0;
		border-bottom-left-radius: 0;
	}

	.version-selector.compact {
		gap: 8px;
	}

	.version-selector.compact .loader-unified {
		justify-content: center;
	}

	.version-selector.compact .loader-btn {
		flex: 0 0 auto;
		padding: 6px 8px;
		font-size: 0.7rem;
		gap: 0;
	}

	.version-selector.compact .loader-btn :global(.icon-svg) {
		width: 16px;
		height: 16px;
		flex-shrink: 0;
	}

	.version-selector.compact .loader-btn span {
		display: inline-block;
		min-width: 0;
		max-width: 0;
		margin-left: 0;
		opacity: 0;
		transition:
			max-width 0.4s cubic-bezier(0.4, 0, 0.2, 1),
			margin-left 0.4s cubic-bezier(0.4, 0, 0.2, 1),
			opacity 0.4s cubic-bezier(0.4, 0, 0.2, 1);
		pointer-events: none;
	}

	.version-selector.compact .loader-btn:hover span,
	.version-selector.compact .loader-btn:focus-visible span {
		max-width: 120px;
		margin-left: 4px;
		opacity: 1;
	}

	.version-selector.compact
		:global(.custom-select-container .select-trigger) {
		padding: 4px 8px;
		font-size: 0.75rem;
		min-height: 0;
	}
</style>
