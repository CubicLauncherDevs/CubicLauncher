<script lang="ts">
	import { onMount } from "svelte";
	import {
		getAvailableVersions,
		getFabricVersions,
		getFabricLoaderVersions,
		getForgeVersions,
		getQuiltVersions,
		getQuiltLoaderVersions,
	} from "$lib/api/cubicApi";
	import Select from "$lib/components/layout/Select.svelte";
	import { t } from "$lib/i18n";
	import { launcherStore } from "$lib/state/state.svelte";
	import { showError } from "$lib/state/state.svelte";
	import type { MinecraftVersion, ForgeGameVersion } from "$lib/types/types";

	let {
		selectedLoader = $bindable<string>("vanilla"),
		selectedMcVersion = $bindable<string>(""),
		selectedLoaderVersion = $bindable<string>(""),
	}: {
		selectedLoader: string;
		selectedMcVersion: string;
		selectedLoaderVersion: string;
	} = $props();

	const LOADERS = [
		{ value: "vanilla", label: "Vanilla", icon: "/images/instances/vanilla.png" },
		{ value: "fabric", label: "Fabric", icon: "/images/instances/fabric.png" },
		{ value: "forge", label: "Forge", icon: "/images/instances/forge.png" },
		{ value: "quilt", label: "Quilt", icon: "/images/instances/quilt.png" },
	];

	let mcVersions = $state<string[]>([]);
	let loaderVersions = $state<string[]>([]);
	let loadingMinecraft = $state(false);
	let loadingLoader = $state(false);

	let vanillaVersions = $state<MinecraftVersion[]>([]);
	let fabricVersions = $state<{ version: string; stable: boolean }[]>([]);
	let quiltVersions = $state<{ version: string; stable: boolean }[]>([]);
	let forgeVersions = $state<ForgeGameVersion[]>([]);

	let mcLoadId = $state(0);
	let loaderLoadId = $state(0);

	async function loadMcVersions(loader: string) {
		selectedLoader = loader;
		const currentLoadId = ++mcLoadId;
		++loaderLoadId; // invalida cualquier carga de loader anterior que aún esté en vuelo
		selectedMcVersion = "";
		selectedLoaderVersion = "";
		loaderVersions = [];
		mcVersions = [];
		loadingMinecraft = true;
		loadingLoader = true;

		try {
			let list: string[] = [];

			if (loader === "vanilla") {
				if (vanillaVersions.length === 0) {
					vanillaVersions = await getAvailableVersions();
				}
				list = vanillaVersions
					.filter((v) => {
						if (v.type === "release") return true;
						if (
							v.type === "snapshot" &&
							launcherStore.settings.show_snapshots
						)
							return true;
						if (
							(v.type === "old_alpha" || v.type === "old_beta") &&
							launcherStore.settings.show_alpha
						)
							return true;
						return false;
					})
					.map((v) => v.id);
			} else if (loader === "fabric") {
				if (fabricVersions.length === 0) {
					fabricVersions = await getFabricVersions();
				}
				list = fabricVersions.map((v) => v.version);
			} else if (loader === "quilt") {
				if (quiltVersions.length === 0) {
					quiltVersions = await getQuiltVersions();
				}
				list = quiltVersions.map((v) => v.version);
			} else if (loader === "forge") {
				if (forgeVersions.length === 0) {
					forgeVersions = await getForgeVersions();
				}
				list = Array.from(
					new Set(forgeVersions.map((v) => v.game_version)),
				);
			}

			if (currentLoadId !== mcLoadId) return;
			mcVersions = list;
			selectedMcVersion = list[0] ?? "";
			await loadLoaderVersions(selectedMcVersion, loader);
		} catch {
			if (currentLoadId !== mcLoadId) return;
			showError("Error", "No se pudieron cargar las versiones de Minecraft");
			mcVersions = [];
			loadingLoader = false;
		} finally {
			if (currentLoadId === mcLoadId) loadingMinecraft = false;
		}
	}

	async function loadLoaderVersions(mcVersion: string, loader: string) {
		const currentLoadId = ++loaderLoadId;
		selectedLoaderVersion = "";
		loaderVersions = [];

		if (!mcVersion || loader === "vanilla") {
			loadingLoader = false;
			return;
		}

		loadingLoader = true;
		try {
			let list: string[] = [];

			if (loader === "fabric") {
				list = await getFabricLoaderVersions(mcVersion);
			} else if (loader === "quilt") {
				list = await getQuiltLoaderVersions(mcVersion);
			} else if (loader === "forge") {
				list = forgeVersions
					.filter((v) => v.game_version === mcVersion)
					.map((v) => v.forge_version);
			}

			if (currentLoadId !== loaderLoadId) return;
			loaderVersions = list;
			selectedLoaderVersion = list[0] ?? "";
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

	const loaderVersionOptions = $derived(
		loaderVersions.map((v) => ({ value: v, label: v })),
	);
</script>

<div class="version-selector">
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
			placeholder={t("createInstance.selectMcVersion")}
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
		transition: background-color 0.15s, color 0.15s, border-color 0.15s,
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
	.linked-selects > :global(.custom-select-container:first-child .select-trigger) {
		border-top-right-radius: 0;
		border-bottom-right-radius: 0;
		border-right: none;
	}

	/* Right select: left radii removed */
	.linked-selects > :global(.custom-select-container:last-child .select-trigger) {
		border-top-left-radius: 0;
		border-bottom-left-radius: 0;
	}
</style>
