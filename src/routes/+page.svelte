<script lang="ts">
	import { onMount, onDestroy } from "svelte";
	import { lazyComponent } from "$lib/utils/lazyComponent.svelte";
	import "../styles/App.css";
	import { launcherStore } from "$lib/state/state.svelte";
	import {
		getVersions,
		syncSettings,
		initEventListeners,
		destroyEventListeners,
	} from "$lib/api/launcherService";
	import type { InstanceDto } from "$lib/types/types";
	import Sidebar from "$lib/components/layout/Sidebar/Sidebar.svelte";
	import SidebarCompact from "$lib/components/layout/Sidebar/SidebarCompact.svelte";
	import InstanceView from "$lib/components/instances/InstanceView/InstanceView.svelte";
	import Drawer from "$lib/components/layout/Drawer.svelte";
	import ModalBase from "$lib/components/layout/ModalBase.svelte";
	import NotificationContainer from "$lib/components/ui/NotificationContainer.svelte";
	import JreInstallPrompt from "$lib/components/ui/JreInstallPrompt.svelte";
	import AccountSetup from "$lib/components/layout/auth/AccountSetup.svelte";
	import { getAccountProvider } from "$lib/utils/accountProviders";
	import { updaterState } from "$lib/state/updaterState.svelte";
	import Icon from "$lib/icons/Icon.svelte";
	import { initDiscordPresence } from "$lib/api/cubicApi";
	import { t } from "$lib/i18n";
	import {
		applyTheme,
		importThemeZip,
		import_theme_cbth,
	} from "$lib/api/themeManager";
	import { startAutoUpdates } from "$lib/api/updaterServices";
	import { saveSettings } from "$lib/api/launcherService";
	import { showSuccess, showError } from "$lib/state/state.svelte";
	import { loadInstalledVersions } from "$lib/state/versionsState.svelte";
	import { animDuration } from "$lib/utils/animations";
	import {
		applyInterfaceScale,
		applyInterfaceDensity,
		applyInterfaceFont,
	} from "$lib/api/interfaceAppearance";
	import { interfacePreferences } from "$lib/utils/interfacePreferences";
	import { injectProjectFonts } from "$lib/fonts/projectFonts";

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
	let pendingModpackName = $state<string | null>(null);
	const selectedInstanceId = $derived(selectedInstance?.uuid);
	const lastSelectedInstanceKey = "lastSelectedInstanceId";
	let sidebarMode = $state<"normal" | "compact">("normal");
	let transitioning = $state(false);
	let quickMenuOpen = $state(false);
	let instanceEditorOpen = $state(false);
	let versionDownloaderOpen = $state(false);
	let openCreateModal = $state(false);
	let showProfileView = $state(false);
	let droppedMrpackPath = $state<string | null>(null);
	let droppedInstanceZipPath = $state<string | null>(null);
	let isDragOver = $state(false);
	let dragPaths = $state<string[]>([]);
	let editingInstance = $state<InstanceDto | null>(null);
	let showTutorial = $state(false);
	const settingsPanel = lazyComponent(
		() => import("$lib/components/settings/Settings.svelte"),
		() => quickMenuOpen,
		(error) => {
			quickMenuOpen = false;
			showError(t("errors.title"), String(error));
		},
	);
	const SettingsComponent = $derived(settingsPanel.component);

	const versionPanel = lazyComponent(
		() =>
			import("$lib/components/layout/VersionDownloader/VersionDownloader.svelte"),
		() => versionDownloaderOpen,
		(error) => {
			versionDownloaderOpen = false;
			showError(t("errors.title"), String(error));
		},
	);
	const VersionDownloaderComponent = $derived(versionPanel.component);

	const createPanel = lazyComponent(
		() =>
			import("$lib/components/instances/CreateInstanceModal/CreateInstanceModal.svelte"),
		() => openCreateModal,
		(error) => {
			openCreateModal = false;
			showError(t("errors.title"), String(error));
		},
	);
	const CreateInstanceModal = $derived(createPanel.component);

	const editorPanel = lazyComponent(
		() =>
			import("$lib/components/instances/InstanceDrawer/InstanceDrawer.svelte"),
		() => instanceEditorOpen,
		(error) => {
			instanceEditorOpen = false;
			showError(t("errors.title"), String(error));
		},
	);
	const InstanceDrawer = $derived(editorPanel.component);

	const profilePanel = lazyComponent(
		() => import("$lib/components/profiles/ProfileView.svelte"),
		() => showProfileView,
		(error) => {
			showProfileView = false;
			showError(t("errors.title"), String(error));
		},
	);
	const ProfileView = $derived(profilePanel.component);

	const tutorialPanel = lazyComponent(
		() => import("$lib/components/layout/welcome/welcome.svelte"),
		() => showTutorial,
		(error) => {
			showTutorial = false;
			showError(t("errors.title"), String(error));
		},
	);
	const Tutorial = $derived(tutorialPanel.component);

	let logError = $state("");
	const logPanel = lazyComponent(
		() => import("$lib/components/log/LogWindow.svelte"),
		() => logParams !== null,
		(error) => {
			logError = String(error);
		},
	);
	const LogWindow = $derived(logPanel.component);

	const updatePanel = lazyComponent(
		() => import("$lib/components/ui/UpdateModal.svelte"),
		() => !logParams && updaterState.open,
		(error) => {
			updaterState.open = false;
			showError(t("errors.title"), String(error));
		},
	);
	const UpdateModal = $derived(updatePanel.component);

	const sidebarTransitionDuration = $derived(animDuration(0.35, 0.05));
	const interfaceScale = $derived(
		interfacePreferences(launcherStore.settings.interface_preferences)
			.scale,
	);
	const interfaceDensity = $derived(
		interfacePreferences(launcherStore.settings.interface_preferences)
			.density,
	);
	const interfaceFont = $derived(
		interfacePreferences(launcherStore.settings.interface_preferences).font,
	);

	$effect(() => {
		void applyInterfaceScale(interfaceScale).catch((error) => {
			showError(t("settings.interface.scaleError"), String(error));
		});
	});

	$effect(() => {
		applyInterfaceDensity(interfaceDensity);
	});

	$effect(() => {
		applyInterfaceFont(interfaceFont);
	});
	onDestroy(() => {
		if (typeof document !== "undefined") applyInterfaceDensity("theme");
	});

	$effect(() => {
		if (typeof document === "undefined") return;
		const html = document.documentElement;
		const s = launcherStore.settings;
		html.toggleAttribute("data-reduce-motion", s.reduce_animations);
		html.toggleAttribute("data-no-blur", s.disable_blur_effects);
		html.toggleAttribute(
			"data-no-infinite-animations",
			s.disable_infinite_animations,
		);
	});

	let unlistenDragDrop: (() => void) | undefined;
	let settingsReady = $state(false);
	let editingTimer: ReturnType<typeof setTimeout> | undefined;
	let destroyed = false;

	$effect(() => {
		if (
			!logParams &&
			settingsReady &&
			launcherStore.settings.auto_updates &&
			!showTutorial
		) {
			return startAutoUpdates();
		}
	});

	onMount(async () => {
		injectProjectFonts();
		initEventListeners(logParams ? "logs" : "main");
		if (logParams) {
			await syncSettings();
			return;
		}

		try {
			if (localStorage.getItem("sidebarMode") === "compact")
				sidebarMode = "compact";
		} catch {
			/* Storage may be unavailable; startup must still continue. */
		}

		// Updates depend on settings, not on a successful instance/version scan.
		const settings = syncSettings()
			.then(() => {
				if (destroyed) return;
				showTutorial =
					launcherStore.settings.show_tutorial ||
					!launcherStore.settings.license_accepted;
				settingsReady = true;
			})
			.catch((error) => {
				console.warn("Could not load startup settings", error);
				if (!destroyed) settingsReady = true;
			});
		await Promise.allSettled([
			settings,
			getVersions(),
			loadInstalledVersions(),
		]);
		if (destroyed) return;

		if (!logParams && !selectedInstance) {
			const storedInstanceId = localStorage.getItem(
				lastSelectedInstanceKey,
			);
			selectedInstance =
				launcherStore.loadedInstances.find(
					(instance) => instance.uuid === storedInstanceId,
				) ??
				launcherStore.loadedInstances[0] ??
				null;
		}

		if (launcherStore.settings.discord_presence) {
			initDiscordPresence();
		}

		setupDragDrop();
	});

	onDestroy(() => {
		destroyed = true;
		destroyEventListeners();
		unlistenDragDrop?.();
		clearTimeout(editingTimer);
	});

	$effect(() => {
		const theme = launcherStore.settings.theme;
		if (theme) {
			applyTheme(theme);
		}
	});

	$effect(() => {
		if (!logParams) localStorage.setItem("sidebarMode", sidebarMode);
	});

	$effect(() => {
		// Track only the UUID so status updates do not trigger storage writes.
		if (!logParams && selectedInstanceId) {
			localStorage.setItem(lastSelectedInstanceKey, selectedInstanceId);
		}
	});

	async function setupDragDrop() {
		try {
			const { getCurrentWebview } =
				await import("@tauri-apps/api/webview");
			if (destroyed) return;
			const webview = getCurrentWebview();
			unlistenDragDrop = await webview.onDragDropEvent((event) => {
				if (destroyed) return;
				if (event.payload.type === "enter") {
					const payload = event.payload as { paths: string[] };
					dragPaths = payload.paths ?? [];
					isDragOver =
						dragPaths.length > 0 &&
						dragPaths.some(
							(p) =>
								p.endsWith(".mrpack") ||
								p.endsWith(".zip") ||
								p.endsWith(".cbth"),
						);
				} else if (event.payload.type === "leave") {
					isDragOver = false;
					dragPaths = [];
				} else if (event.payload.type === "drop") {
					isDragOver = false;
					const paths =
						(event.payload as { paths: string[] }).paths ?? [];
					const zipFile = paths.find((p: string) =>
						p.endsWith(".zip"),
					);
					const mrpackFile = paths.find((p: string) =>
						p.endsWith(".mrpack"),
					);
					const cbth = paths.find((p) => p.endsWith(".cbth"));
					if (zipFile) {
						handleZipDrop(zipFile);
					} else if (mrpackFile) {
						droppedMrpackPath = mrpackFile;
						droppedInstanceZipPath = null;
						openCreateModal = true;
					} else if (cbth) {
						handleCbthDrop(cbth);
					}
					dragPaths = [];
				}
			});
			if (destroyed) unlistenDragDrop();
		} catch (e) {
			console.warn("Drag-drop not available:", e);
		}
	}

	async function handleZipDrop(zipPath: string) {
		try {
			await importThemeZip(zipPath);
			showSuccess(
				t("themes.importSuccess"),
				t("themes.importSuccessMessage"),
			);
		} catch (e) {
			const msg = String(e);
			if (
				msg.includes("no se encontró theme.json") ||
				msg.includes("no theme.json") ||
				msg.includes("no se encontró Meta.toml")
			) {
				droppedMrpackPath = null;
				droppedInstanceZipPath = zipPath;
				openCreateModal = true;
			} else {
				showError(t("themes.importError"), msg);
			}
		}
	}

	async function handleCbthDrop(cbthPath: string) {
		try {
			await import_theme_cbth(cbthPath);
			showSuccess(
				t("themes.importSuccess"),
				t("themes.importSuccessMessage"),
			);
		} catch (e) {
			const msg = String(e);
			if (
				msg.includes("no se encontró theme.json") ||
				msg.includes("no theme.json") ||
				msg.includes("no se encontró Meta.toml")
			) {
				droppedMrpackPath = cbthPath;
				openCreateModal = true;
			} else {
				showError(t("themes.importError"), msg);
			}
		}
	}

	function toggleSidebar() {
		if (transitioning) return;
		transitioning = true;
		sidebarMode = sidebarMode === "normal" ? "compact" : "normal";
		setTimeout(() => {
			transitioning = false;
		}, 350);
	}

	function onTutorialClose() {
		launcherStore.settings.show_tutorial = false;
		saveSettings();
	}

	function selectPendingModpack() {
		if (!pendingModpackName) return;
		const instance = launcherStore.loadedInstances.find(
			(instance) => instance.name === pendingModpackName,
		);
		if (instance) {
			selectedInstance = instance;
			showProfileView = false;
			pendingModpackName = null;
		}
	}

	$effect(() => {
		selectPendingModpack();
	});

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
					updated.last_played !== sel.last_played ||
					updated.playtime_seconds !== sel.playtime_seconds ||
					updated.icon !== sel.icon)
			) {
				selectedInstance = updated;
			} else if (!updated) {
				selectedInstance = null;
			}
		}
	});

	$effect(() => {
		const instances = launcherStore.loadedInstances;
		const ei = editingInstance;
		if (ei) {
			const updated = instances.find((i) => i.uuid === ei.uuid);
			if (updated && updated.overrides !== ei.overrides) {
				editingInstance = updated;
			} else if (!updated) {
				editingInstance = null;
			}
		}
	});
</script>

{#if logParams}
	{#if LogWindow}
		<LogWindow
			instanceId={logParams.instanceId}
			instanceName={logParams.instanceName}
		/>
	{:else if logError}
		<p role="alert">{t("errors.title")}: {logError}</p>
	{:else}
		<p role="status">{t("common.loading")}</p>
	{/if}
{:else}
	<div class="app-container" class:drag-over={isDragOver}>
		{#if isDragOver}
			<div class="drag-overlay">
				<div class="drag-overlay-content">
					<div class="drag-icon">
						<Icon name="instance:box" size={48} />
					</div>
					<h2>Suelta tu modpack o theme aquí</h2>
					<p>
						Los archivos .mrpack y .zip se importarán
						automáticamente
					</p>
				</div>
			</div>
		{/if}

		<div
			class="sidebar-container"
			class:compact={sidebarMode === "compact"}
			style="transition-duration: {sidebarTransitionDuration}s"
		>
			{#if sidebarMode === "normal"}
				<Sidebar
					bind:selectedInstance
					onopenquickmenu={() => (quickMenuOpen = true)}
					onopenprofileview={() => (showProfileView = true)}
					onopenversiondownloader={() =>
						(versionDownloaderOpen = true)}
					onopencreateinstance={() => (openCreateModal = true)}
					onopeneditinstance={(inst) => {
						instanceEditorOpen = true;
						editingInstance = inst;
					}}
					oncollapse={toggleSidebar}
					onselectinstance={() => (showProfileView = false)}
				/>
			{:else}
				<SidebarCompact
					bind:selectedInstance
					onopenquickmenu={() => (quickMenuOpen = true)}
					onopenprofileview={() => (showProfileView = true)}
					onopenversiondownloader={() =>
						(versionDownloaderOpen = true)}
					onopencreateinstance={() => (openCreateModal = true)}
					onopeneditinstance={(inst) => {
						instanceEditorOpen = true;
						editingInstance = inst;
					}}
					onexpand={toggleSidebar}
					onselectinstance={() => (showProfileView = false)}
				/>
			{/if}
		</div>

		<main class="main-content">
			<div class="background-overlay"></div>

			{#if showProfileView}
				{#if ProfileView}
					<ProfileView onclose={() => (showProfileView = false)} />
				{:else}
					<p role="status">{t("common.loading")}</p>
				{/if}
			{:else if selectedInstance}
				<InstanceView {selectedInstance} />
			{:else}
				<div class="empty-state">
					<div class="empty-logo" aria-label="Cubic"></div>
					<h2>{t("main.noInstanceTitle")}</h2>
					<p>{t("main.noInstanceDesc")}</p>
				</div>
			{/if}
		</main>
	</div>

	<Drawer bind:open={quickMenuOpen} direction="right">
		{#if SettingsComponent}
			<SettingsComponent onclose={() => (quickMenuOpen = false)} />
		{:else}
			<p role="status">{t("common.loading")}</p>
		{/if}
	</Drawer>

	{#if editingInstance}
		<Drawer bind:open={instanceEditorOpen} direction="right">
			{#if InstanceDrawer}
				<InstanceDrawer
					onclose={() => {
						instanceEditorOpen = false;
						clearTimeout(editingTimer);
						editingTimer = setTimeout(
							() => (editingInstance = null),
							350,
						);
					}}
					instance={editingInstance}
				/>
			{:else}
				<p role="status">{t("common.loading")}</p>
			{/if}
		</Drawer>
	{/if}

	{#if VersionDownloaderComponent}
		<VersionDownloaderComponent bind:open={versionDownloaderOpen} />
	{:else if versionDownloaderOpen}
		<ModalBase
			bind:open={versionDownloaderOpen}
			title={t("versionDownloader.title")}
			width="800px"
		>
			<p role="status">{t("versionDownloader.loading")}</p>
		</ModalBase>
	{/if}

	{#if CreateInstanceModal}
		<CreateInstanceModal
			bind:open={openCreateModal}
			bind:mrpackPath={droppedMrpackPath}
			bind:instanceZipPath={droppedInstanceZipPath}
			oninstallstarted={(name) => (pendingModpackName = name)}
			oninstallfailed={(name) => {
				if (pendingModpackName === name) pendingModpackName = null;
			}}
		/>
	{:else if openCreateModal}
		<ModalBase
			bind:open={openCreateModal}
			title={t("createInstance.title")}
		>
			<p role="status">{t("common.loading")}</p>
		</ModalBase>
	{/if}

	{#if Tutorial}
		<Tutorial
			bind:open={showTutorial}
			onclose={onTutorialClose}
			onopensettings={() => (quickMenuOpen = true)}
		/>
	{/if}

	<NotificationContainer />
	<JreInstallPrompt />
	{#if launcherStore.reauthPrompt && !logParams}
		<AccountSetup
			provider={getAccountProvider(launcherStore.reauthPrompt)}
			initialServerUrl={launcherStore.reauthPrompt.yggdrasil_server_url ??
				undefined}
			autoConnect={launcherStore.reauthPrompt.user_type === "Yggdrasil"}
			reauthUsername={launcherStore.reauthPrompt.username}
			onclose={() => (launcherStore.reauthPrompt = null)}
		/>
	{/if}
	{#if UpdateModal}
		<UpdateModal />
	{:else if updaterState.open}
		<ModalBase
			bind:open={updaterState.open}
			title={t("updater.title")}
			width="560px"
		>
			<p role="status">{t("common.loading")}</p>
		</ModalBase>
	{/if}
{/if}

<style>
	.drag-overlay {
		position: fixed;
		inset: 0;
		z-index: 9999;
		background: var(--bg-overlay);
		display: flex;
		align-items: center;
		justify-content: center;
		backdrop-filter: blur(var(--backdrop-blur-overlay, 4px));
	}

	.drag-overlay-content {
		text-align: center;
		color: var(--media-overlay-text);
	}

	.drag-overlay-content .drag-icon {
		display: flex;
		justify-content: center;
		align-items: center;
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

	.sidebar-container {
		flex-shrink: 0;
		overflow: visible;
		display: flex;
		width: var(--sidebar-width);
		background: var(--bg-sidebar-gradient, var(--bg-sidebar));
		transition: width 0.35s cubic-bezier(0.32, 0.72, 0, 1);
	}

	.sidebar-container.compact {
		width: 70px;
	}

	.empty-state {
		flex: 1;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 8px;
	}

	.empty-logo {
		width: 120px;
		height: 120px;
		opacity: 0.5;
		background: var(--cubic-logo);
		background-size: contain;
		background-repeat: no-repeat;
		background-position: center;
	}
</style>
