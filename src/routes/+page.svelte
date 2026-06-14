<script lang="ts">
	import { onMount } from "svelte";
	import type { Component } from "svelte";
	import "../styles/App.css";
	import { launcherStore } from "$lib/state/state.svelte";
	import {
		getVersions,
		syncSettings,
		initEventListeners,
	} from "$lib/api/launcherService";
	import type { InstanceDto } from "$lib/types/types";
	import Sidebar from "$lib/components/layout/Sidebar.svelte";
	import InstanceView from "$lib/components/instances/InstanceView.svelte";
	import Drawer from "$lib/components/layout/Drawer.svelte";
	import NotificationContainer from "$lib/components/ui/NotificationContainer.svelte";
	import Tutorial from "$lib/components/layout/welcome.svelte";
	import { initDiscordPresence } from "$lib/api/cubicApi";
	import { t } from "$lib/i18n";
	import { applyTheme } from "$lib/api/themeManager";
	import { checkForUpdates } from "$lib/api/updaterServices";
	import { saveSettings } from "$lib/api/launcherService";
	import CreateInstanceModal from "$lib/components/instances/CreateInstanceModal.svelte";
	import LogWindow from "$lib/components/log/LogWindow.svelte";

	const logParams = $derived.by(() => {
		if (typeof window === "undefined") return null;
		const params = new URLSearchParams(window.location.search);
		const logId = params.get("log");
		if (!logId) return null;
		return {
			instanceId: decodeURIComponent(logId),
			instanceName: decodeURIComponent(params.get("name") || "Logs"),
		};
	});

	let selectedInstance = $state<InstanceDto | null>(null);
	let quickMenuOpen = $state(false);
	let versionDownloaderOpen = $state(false);
	let openCreateModal = $state(false);
	let droppedMrpackPath = $state<string | null>(null);
	let isDragOver = $state(false);

	let showTutorial = $state(false);
	let SettingsComponent = $state<Component<{ onclose: () => void }> | null>(
		null,
	);
	let VersionDownloaderComponent = $state<Component<{
		onclose?: () => void;
	}> | null>(null);

	onMount(async () => {
		initEventListeners();

		await Promise.all([syncSettings(), getVersions()]);

		if (launcherStore.settings.show_tutorial) {
			showTutorial = true;
		}

		const firstInstance = launcherStore.loadedInstances[0];
		if (firstInstance && !selectedInstance) {
			selectedInstance = firstInstance;
		}

		applyTheme(launcherStore.settings.theme);

		initDiscordPresence();

		if (launcherStore.settings.auto_updates) {
			setTimeout(() => checkForUpdates(true), 2000);
		}

		// Lazy load non-critical components after first paint
		Promise.all([
			import("$lib/components/settings/Settings.svelte"),
			import("$lib/components/layout/VersionDownloader.svelte"),
		]).then(([s, v]) => {
			SettingsComponent = s.default;
			VersionDownloaderComponent = v.default;
		});

		setupDragDrop();
	});

	async function setupDragDrop() {
		try {
			const { getCurrentWebview } = await import("@tauri-apps/api/webview");
			const webview = getCurrentWebview();
			await webview.onDragDropEvent((event) => {
				if (event.payload.type === "over") {
					const payload = event.payload as { paths?: string[]; position: { x: number; y: number } };
					isDragOver =
						!!payload.paths &&
						payload.paths.length > 0 &&
						payload.paths.some(
							(p) =>
								p.endsWith(".mrpack") || p.endsWith(".zip"),
						);
				} else if (event.payload.type === "leave") {
					isDragOver = false;
				} else if (event.payload.type === "drop") {
					isDragOver = false;
					const paths = event.payload.paths as string[];
					const mrpackFile = paths.find(
						(p: string) =>
							p.endsWith(".mrpack") || p.endsWith(".zip"),
					);
					if (mrpackFile) {
						droppedMrpackPath = mrpackFile;
						openCreateModal = true;
					}
				}
			});
		} catch (e) {
			console.warn("Drag-drop not available, falling back:", e);
		}
	}

	function onTutorialClose() {
		launcherStore.settings.show_tutorial = false;
		saveSettings();
	}

	$effect(() => {
		const instances = launcherStore.loadedInstances;
		const sel = selectedInstance;
		if (sel) {
			const updated = instances.find((i) => i.uuid === sel.uuid);
			if (
				updated &&
				(updated.status !== sel.status ||
					updated.name !== sel.name ||
					updated.loader !== sel.loader ||
					updated.version !== sel.version ||
					updated.last_played !== sel.last_played)
			) {
				selectedInstance = updated;
			}
		}
	});
</script>

{#if logParams}
	<LogWindow instanceId={logParams.instanceId} instanceName={logParams.instanceName} />
{:else}
<div class="app-container" class:drag-over={isDragOver}>
	{#if isDragOver}
		<div class="drag-overlay">
			<div class="drag-overlay-content">
				<span>📦</span>
				<h2>Suelta tu modpack</h2>
				<p>Los archivos .mrpack se importarán automáticamente</p>
			</div>
		</div>
	{/if}

	<Sidebar
		bind:selectedInstance
		onopenquickmenu={() => (quickMenuOpen = true)}
		onopenversiondownloader={() => (versionDownloaderOpen = true)}
		onopencreateinstance={() => (openCreateModal = true)}
	/>

	<main class="main-content">
		<div class="background-overlay"></div>

		{#if selectedInstance}
			<InstanceView {selectedInstance} />
		{:else}
			<div class="empty-state">
				<img
					src="/images/cubic.svg"
					alt="Cubic"
					style="width: 120px; opacity: 0.1; filter: grayscale(1);"
				/>
				<h2>{t("main.noInstanceTitle")}</h2>
				<p>{t("main.noInstanceDesc")}</p>
			</div>
		{/if}
	</main>
</div>

<Drawer bind:open={quickMenuOpen} direction="right">
	<SettingsComponent onclose={() => (quickMenuOpen = false)} />
</Drawer>

<Drawer bind:open={versionDownloaderOpen} direction="right">
	<VersionDownloaderComponent
		onclose={() => (versionDownloaderOpen = false)}
	/>
</Drawer>

<CreateInstanceModal bind:open={openCreateModal} bind:mrpackPath={droppedMrpackPath} />

<Tutorial bind:open={showTutorial} onclose={onTutorialClose} onopensettings={() => (quickMenuOpen = true)} />

<NotificationContainer />
{/if}

<style>
	.drag-overlay {
		position: fixed;
		inset: 0;
		z-index: 9999;
		background: rgba(0, 0, 0, 0.7);
		display: flex;
		align-items: center;
		justify-content: center;
		backdrop-filter: blur(4px);
	}

	.drag-overlay-content {
		text-align: center;
		color: white;
	}

	.drag-overlay-content span {
		font-size: 3rem;
		display: block;
		margin-bottom: 16px;
	}

	.drag-overlay-content h2 {
		font-size: 1.5rem;
		margin-bottom: 8px;
	}

	.drag-overlay-content p {
		font-size: 0.9rem;
		opacity: 0.8;
	}
</style>
