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

	let {
		selectedLoader = $bindable<string>("vanilla"),
		selectedMcVersion = $bindable<string>(""),
		selectedLoaderVersion = $bindable<string>(""),
		compact = false,
	}: {
		selectedLoader: string;
		selectedMcVersion: string;
		selectedLoaderVersion: string;
		compact?: boolean;
	} = $props();

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
		} else if (loader === "neoforge") {
			baseVersions = Array.from(parsed.neoforge).map((v) => {
				const idx = v.indexOf("-neoforge-");
				return idx >= 0 ? v.substring(0, idx) : v;
			});
		}

		return Array.from(new SvelteSet(baseVersions)).sort(compareVersions);
	}

	const availableMcVersions = $derived(
		getMcVersionsForLoader(selectedLoader),
	);

	const mcVersionOptions = $derived(
		availableMcVersions.map((v) => ({ value: v, label: v })),
	);

	const mcPlaceholder = $derived(
		!versionsState.loading && availableMcVersions.length === 0
			? t("createInstance.noVersionsErr")
			: t("createInstance.selectMcVersion"),
	);

	const availableLoaderVersions = $derived.by(() => {
		if (!selectedMcVersion || selectedLoader === "vanilla") return [];
		const key = `${selectedLoader}:${selectedMcVersion}`;
		const installed =
			versionsState.loaderVersions?.get(key) ?? new Set<string>();
		return Array.from(installed).sort(compareVersions);
	});

	const loaderVersionOptions = $derived(
		availableLoaderVersions.map((v) => ({ value: v, label: v })),
	);

	$effect(() => {
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

<div class="version-selector" class:compact>
	<div class="loader-unified">
		{#each LOADERS as loader (loader.value)}
			<button
				type="button"
				class="loader-btn"
				class:active={selectedLoader === loader.value}
				onclick={() => (selectedLoader = loader.value)}
			>
				<Icon name={loader.iconName} size={20} />
				<span>{loader.label}</span>
			</button>
		{/each}
	</div>

	<div class="linked-selects">
		<Select
			bind:value={selectedMcVersion}
			options={mcVersionOptions}
			placeholder={mcPlaceholder}
			loading={versionsState.loading}
			loadingPlaceholder={t("createInstance.loading")}
			disabled={versionsState.loading || mcVersionOptions.length === 0}
		/>

		<Select
			bind:value={selectedLoaderVersion}
			options={loaderVersionOptions}
			placeholder={selectedLoader === "vanilla"
				? t("createInstance.noLoader")
				: t("createInstance.selectLoaderVersion")}
			loading={versionsState.loading}
			loadingPlaceholder={t("createInstance.loading")}
			disabled={selectedLoader === "vanilla" ||
				versionsState.loading ||
				loaderVersionOptions.length === 0}
		/>
	</div>
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

	.version-selector.compact .loader-btn {
		padding: 6px 4px;
		font-size: 0.7rem;
		gap: 4px;
	}

	.version-selector.compact .loader-btn :global(.icon-svg) {
		width: 16px;
		height: 16px;
	}

	.version-selector.compact
		:global(.custom-select-container .select-trigger) {
		padding: 4px 8px;
		font-size: 0.75rem;
		min-height: 0;
	}
</style>
