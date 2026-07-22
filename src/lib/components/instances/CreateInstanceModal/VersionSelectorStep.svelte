<script lang="ts">
	import { onMount } from "svelte";
	import { SvelteSet } from "svelte/reactivity";
	import {
		getInstalledVersions,
		getInstalledMcVersions,
		getInstalledLoaderVersions,
	} from "$lib/api/cubicApi";
	import Select from "$lib/components/layout/Select.svelte";
	import { t } from "$lib/i18n";
	import { showError } from "$lib/state/state.svelte";

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

	const LOADERS = [
		{
			value: "vanilla",
			label: "Vanilla",
			icon: "/images/instances/vanilla.png",
		},
		{
			value: "fabric",
			label: "Fabric",
			icon: "/images/instances/fabric.png",
		},
		{ value: "forge", label: "Forge", icon: "/images/instances/forge.png" },
		{
			value: "neoforge",
			label: "NeoForge",
			icon: "/images/instances/neoforged.png",
		},
		{ value: "quilt", label: "Quilt", icon: "/images/instances/quilt.png" },
	];

	let mcVersions = $state<string[]>([]);
	let loaderVersions = $state<string[]>([]);
	let loadingMinecraft = $state(false);
	let loadingLoader = $state(false);

	let cachedInstalledVersions: string[] | null = null;
	let cachedParsedVersions: ReturnType<typeof getInstalledMcVersions> | null =
		null;
	let cachedInstalledLoaderVersions: Map<string, Set<string>> | null = null;

	let mcLoadId = $state(0);
	let loaderLoadId = $state(0);

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

	async function loadMcVersions(loader: string) {
		selectedLoader = loader;
		const currentLoadId = ++mcLoadId;
		++loaderLoadId;
		loadingMinecraft = true;
		loadingLoader = true;

		try {
			if (cachedInstalledVersions === null) {
				cachedInstalledVersions = await getInstalledVersions();
				cachedParsedVersions = getInstalledMcVersions(
					cachedInstalledVersions,
				);
				cachedInstalledLoaderVersions = getInstalledLoaderVersions(
					cachedInstalledVersions,
				);
			}
			const parsed = cachedParsedVersions!;

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

			const deduped = Array.from(new SvelteSet(baseVersions)).sort(
				compareVersions,
			);

			if (currentLoadId !== mcLoadId) return;
			mcVersions = deduped;
			if (!selectedMcVersion || !deduped.includes(selectedMcVersion)) {
				selectedMcVersion = deduped[0] ?? "";
			}
			await loadLoaderVersions(selectedMcVersion, loader);
		} catch {
			if (currentLoadId !== mcLoadId) return;
			showError(
				"Error",
				"No se pudieron cargar las versiones de Minecraft",
			);
			loadingLoader = false;
		} finally {
			if (currentLoadId === mcLoadId) loadingMinecraft = false;
		}
	}

	async function loadLoaderVersions(mcVersion: string, loader: string) {
		const currentLoadId = ++loaderLoadId;

		if (!mcVersion || loader === "vanilla") {
			loaderVersions = [];
			selectedLoaderVersion = "";
			loadingLoader = false;
			return;
		}

		loadingLoader = true;
		try {
			const key = `${loader}:${mcVersion}`;
			const installed =
				cachedInstalledLoaderVersions?.get(key) ?? new Set<string>();
			const list = Array.from(installed).sort(compareVersions);

			if (currentLoadId !== loaderLoadId) return;
			loaderVersions = list;
			if (
				!selectedLoaderVersion ||
				!list.includes(selectedLoaderVersion)
			) {
				selectedLoaderVersion = list[0] ?? "";
			}
		} catch {
			if (currentLoadId !== loaderLoadId) return;
			showError(
				"Error",
				`No se pudieron cargar las versiones del loader para ${mcVersion}`,
			);
			loaderVersions = [];
		} finally {
			if (currentLoadId === loaderLoadId) loadingLoader = false;
		}
	}

	onMount(() => {
		loadMcVersions(selectedLoader);
	});

	const mcVersionOptions = $derived(
		mcVersions.map((v) => ({ value: v, label: v })),
	);

	const mcPlaceholder = $derived(
		!loadingMinecraft && mcVersions.length === 0
			? t("createInstance.noVersionsErr")
			: t("createInstance.selectMcVersion"),
	);

	const loaderVersionOptions = $derived(
		loaderVersions.map((v) => ({ value: v, label: v })),
	);
</script>

<div class="version-selector" class:compact>
	<div class="loader-unified">
		{#each LOADERS as loader (loader.value)}
			<button
				type="button"
				class="loader-btn"
				class:active={selectedLoader === loader.value}
				onclick={() => loadMcVersions(loader.value)}
			>
				<img src={loader.icon} alt={loader.label} />
				<span>{loader.label}</span>
			</button>
		{/each}
	</div>

	<div class="linked-selects">
		<Select
			bind:value={selectedMcVersion}
			options={mcVersionOptions}
			placeholder={mcPlaceholder}
			loading={loadingMinecraft}
			loadingPlaceholder={t("createInstance.loading")}
			disabled={loadingMinecraft || mcVersionOptions.length === 0}
			onchange={(value) => loadLoaderVersions(value, selectedLoader)}
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

	.loader-btn img {
		width: 20px;
		height: 20px;
		object-fit: contain;
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

	.version-selector.compact .loader-btn img {
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
