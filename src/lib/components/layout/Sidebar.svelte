<script lang="ts">
	import { onMount } from "svelte";
	import { slide } from "svelte/transition";
	import { SvelteMap } from "svelte/reactivity";
	import { listen } from "@tauri-apps/api/event";
	import { getInstalledVersions, getDownloadQueue } from "$lib/api/cubicApi";
	import { INSTANCE_LOGOS } from "$lib/icons/logos";
	import { deleteInst, updateInst } from "$lib/api/launcherService";
	import { launcherStore } from "$lib/state/state.svelte";
	import type { InstanceDto, AppEvent } from "$lib/types/types";
	import UserMenu from "./UserMenu.svelte";
	import ModalBase from "./ModalBase.svelte";
	import Select from "./Select.svelte";
	import CollapsibleSection from "$lib/components/settings/CollapsibleSection.svelte";
	import { t } from "$lib/i18n";

	interface Props {
		selectedInstance: InstanceDto | null;
		onopenquickmenu?: () => void;
		onopenversiondownloader?: () => void;
		onopencreateinstance?: () => void;
	}

	let {
		selectedInstance = $bindable(),
		onopenquickmenu,
		onopenversiondownloader,
		onopencreateinstance,
	}: Props = $props();

	let showUserMenu = $state(false);
	let showRenameModal = $state(false);
	let showDeleteModal = $state(false);
	let instanceToActOn = $state<InstanceDto | null>(null);
	let renameInput = $state("");
	let versionInput = $state("");
	let selectedIcon = $state<string | null>(null);
	let installedVersions = $state<string[]>([]);
	let availableIcons = $state<string[]>(INSTANCE_LOGOS);
	let versionOptions = $derived(
		installedVersions.map((v) => ({ value: v, label: v })),
	);

	// ── Download Queue ────────────────────────────────────────────────────

	type SegKey = "Library" | "Asset" | "Native" | "Client";
	const SEGS: SegKey[] = ["Library", "Asset", "Native", "Client"];

	interface SegProg {
		current: number;
		total: number;
	}
	interface DlItem {
		version: string;
		activeType: SegKey | null;
		segs: Record<SegKey, SegProg>;
		done: boolean;
	}

	function emptySegs(): Record<SegKey, SegProg> {
		return {
			Library: { current: 0, total: 0 },
			Asset: { current: 0, total: 0 },
			Native: { current: 0, total: 0 },
			Client: { current: 0, total: 0 },
		};
	}

	let downloads = new SvelteMap<string, DlItem>();
	let downloadsOpen = $state(false);
	let activeCount = $derived(
		[...downloads.values()].filter((d) => !d.done).length,
	);
	let doneCount = $derived(
		[...downloads.values()].filter((d) => d.done).length,
	);
	function pct(segs: Record<SegKey, SegProg>): number {
		const totalAll = SEGS.reduce((a, k) => a + segs[k].total, 0);
		const curAll = SEGS.reduce((a, k) => a + segs[k].current, 0);
		return totalAll > 0 ? Math.round((curAll / totalAll) * 100) : 0;
	}

	function toggleDownloads() {
		downloadsOpen = !downloadsOpen;
	}

	onMount(() => {
		getDownloadQueue().then((queue) => {
			for (const item of queue) {
				if (!downloads.has(item.version)) {
					downloads.set(item.version, {
						version: item.version,
						activeType: null,
						segs: emptySegs(),
						done: item.status === "done",
					});
				}
			}
			if (queue.length > 0) {
				downloadsOpen = true;
			}
		});

		const unlisten = listen<AppEvent>("app-event", (event) => {
			const p = event.payload;
			switch (p.type) {
				case "DEnqueue": {
					const { version } = p.data;
					if (!downloads.has(version)) {
						downloads.set(version, {
							version,
							activeType: null,
							segs: emptySegs(),
							done: false,
						});
						downloadsOpen = true;
					}
					break;
				}
				case "DProgress": {
					const { version, current, total, d_type } = p.data;
					const existing = downloads.get(version) ?? {
						version,
						activeType: null,
						segs: emptySegs(),
						done: false,
					};
					const key = SEGS.includes(d_type as SegKey)
						? (d_type as SegKey)
						: (existing.activeType ?? "Library");
					const newSegs = {
						...existing.segs,
						[key]: { current, total },
					};
					downloads.set(version, {
						...existing,
						segs: newSegs,
						activeType: key,
						done: false,
					});
					break;
				}
				case "DFinish": {
					const { version } = p.data;
					const item = downloads.get(version);
					if (item) {
						downloads.set(version, {
							...item,
							done: true,
							activeType: null,
						});
					}
					setTimeout(() => {
						downloads.delete(version);
					}, 4000);
					break;
				}
			}
		});

		return () => {
			unlisten.then((u) => u());
		};
	});

	// ── Instance CRUD ─────────────────────────────────────────────────────

	async function openRenameModal(instance: InstanceDto) {
		instanceToActOn = instance;
		renameInput = instance.name;
		versionInput = instance.version;
		selectedIcon = instance.icon;
		installedVersions = await getInstalledVersions();
		showRenameModal = true;
	}

	function openDeleteModal(instance: InstanceDto) {
		instanceToActOn = instance;
		showDeleteModal = true;
	}

	async function handleRename() {
		if (!instanceToActOn) return;
		const nameChanged = renameInput && renameInput !== instanceToActOn.name;
		const versionChanged =
			versionInput && versionInput !== instanceToActOn.version;

		const iconChanged = selectedIcon !== instanceToActOn.icon;

		if (nameChanged || versionChanged || iconChanged) {
			await updateInst(
				instanceToActOn.uuid,
				nameChanged ? renameInput : undefined,
				versionChanged ? versionInput : undefined,
				iconChanged ? selectedIcon : undefined,
			);

			if (selectedInstance?.uuid === instanceToActOn.uuid) {
				if (nameChanged) selectedInstance.name = renameInput;
				if (versionChanged) selectedInstance.version = versionInput;
				if (iconChanged) selectedInstance.icon = selectedIcon;
			}
		}
		showRenameModal = false;
	}

	async function handleDelete() {
		if (!instanceToActOn) return;
		await deleteInst(instanceToActOn.uuid);
		if (selectedInstance?.uuid === instanceToActOn.uuid) {
			selectedInstance = null;
		}
		showDeleteModal = false;
	}
</script>

<aside class="sidebar">
	<div class="sidebar-header">
		<h1 style="font-size: 0.9rem; font-weight: bold;">CUBICLAUNCHER</h1>
	</div>

	<div class="sidebar-content">
		<div class="section-label">{t("sidebar.yourInstances")}</div>
		<div class="instance-list">
			{#each launcherStore.loadedInstances as instance (instance.uuid)}
				<div
					class="instance-item"
					class:active={selectedInstance?.uuid === instance.uuid}
					onclick={() => (selectedInstance = instance)}
					onkeydown={(e) => {
						if (e.key === "Enter" || e.key === " ")
							selectedInstance = instance;
					}}
					role="button"
					tabindex="0"
					title={instance.name}
				>
					<div class="instance-info-container">
						<div class="instance-icon">
							{#if instance.icon}
								<img
									src={instance.icon}
									alt={instance.name}
									width="16"
									height="16"
								/>
							{:else}
								{instance.name.charAt(0).toUpperCase()}
							{/if}
						</div>
						<span class="instance-name">{instance.name}</span>
					</div>
					<div class="instance-actions">
						<button
							type="button"
							class="action-btn"
							onclick={(e) => {
								e.stopPropagation();
								openRenameModal(instance);
							}}
							title={t("sidebar.rename")}
						>
							<img
								src="/images/icons/edit.svg"
								alt={t("sidebar.rename")}
								width="12"
								height="12"
								style="filter: var(--icon-filter);"
							/>
						</button>
						<button
							type="button"
							class="action-btn delete"
							onclick={(e) => {
								e.stopPropagation();
								openDeleteModal(instance);
							}}
							title={t("sidebar.delete")}
						>
							<img
								src="/images/icons/trash.svg"
								alt={t("sidebar.delete")}
								width="12"
								height="12"
								style="filter: var(--icon-filter-error);"
							/>
						</button>
					</div>
				</div>
			{/each}
			{#if launcherStore.loadedInstances.length === 0}
				<div
					class="instance-item"
					style="opacity: 0.4; cursor: default;"
				>
					<span class="instance-name">{t("sidebar.noInstances")}</span
					>
				</div>
			{/if}
		</div>
	</div>

	<div class="sidebar-sections">
		<div class="sd-root">
			<button
				type="button"
				class="sd-header"
				class:expanded={downloadsOpen}
				onclick={toggleDownloads}
				aria-expanded={downloadsOpen}
			>
				<span class="sd-header-left">
					{#if activeCount > 0}
						<span class="sd-spinner"></span>
						<span class="sd-label"
							>{activeCount} {t("sidebar.downloading")}</span
						>
					{:else if doneCount > 0}
						<svg
							width="12"
							height="12"
							viewBox="0 0 24 24"
							fill="none"
							stroke="var(--color-success)"
							stroke-width="2.5"
							stroke-linecap="round"
							stroke-linejoin="round"
							><polyline points="20 6 9 17 4 12" /></svg
						>
						<span class="sd-label"
							>{doneCount} {t("sidebar.completed")}</span
						>
					{:else}
						<svg
							width="18"
							height="18"
							viewBox="0 0 24 24"
							fill="none"
							stroke="currentColor"
							stroke-width="2"
							stroke-linecap="round"
							stroke-linejoin="round"
						>
							<path
								d="M12 15V3m0 12l-4-4m4 4l4-4M2 17l.621 2.485A2 2 0 0 0 4.561 21h14.878a2 2 0 0 0 1.94-1.515L22 17"
							/>
						</svg>
						<span class="sd-label">{t("sidebar.noDownloads")}</span>
					{/if}
				</span>
				<svg
					class="sd-chevron"
					class:open={downloadsOpen}
					width="16"
					height="16"
					viewBox="0 0 24 24"
					fill="none"
					stroke="currentColor"
					stroke-width="2"
					stroke-linecap="round"
					stroke-linejoin="round"
				>
					<path d="M6 9l6 6 6-6" />
				</svg>
			</button>
			{#if downloadsOpen}
				<div class="sd-body" transition:slide={{ duration: 150 }}>
					{#if downloads.size === 0}
						<div class="sd-empty">
							{t("sidebar.noDownloadDesc")}
						</div>
					{:else}
						{#each [...downloads.values()] as item (item.version)}
							{@const overall = pct(item.segs)}
							<div class="sd-item" class:done={item.done}>
								<div class="sd-item-header">
									<span class="sd-item-left">
										{#if item.done}
											<svg
												width="8"
												height="8"
												viewBox="0 0 24 24"
												fill="none"
												stroke="var(--color-success)"
												stroke-width="3"
												stroke-linecap="round"
												stroke-linejoin="round"
												><polyline
													points="20 6 9 17 4 12"
												/></svg
											>
										{:else}
											<span class="sd-spinner-sm"></span>
										{/if}
										<span class="sd-version"
											>{item.version}</span
										>
									</span>
									<span class="sd-pct" class:done={item.done}
										>{overall}%</span
									>
								</div>
								<div class="sd-progress-track">
									<div
										class="sd-progress-fill"
										class:done={item.done}
										style:width="{overall}%"
									></div>
								</div>
							</div>
						{/each}
					{/if}
				</div>
			{/if}
		</div>

		<div class="section-full">
			<CollapsibleSection
				title={t("sidebar.tools")}
				iconSrc="/images/icons/sliders.svg"
				storageKey="sidebar-tools"
			>
				<div class="tools-group">
					<button
						type="button"
						class="tools-btn"
						onclick={onopencreateinstance}
					>
						<img
							src="/images/icons/create.svg"
							alt=""
							width="14"
							height="14"
						/>
						{t("sidebar.createInstance")}
					</button>
					<button
						type="button"
						class="tools-btn"
						onclick={onopenversiondownloader}
					>
						<img
							src="/images/icons/download.svg"
							alt=""
							width="14"
							height="14"
						/>
						{t("sidebar.downloadVersions")}
					</button>
					<button
						type="button"
						class="tools-btn"
						onclick={onopenquickmenu}
					>
						<img
							src="/images/icons/settings.svg"
							alt=""
							width="14"
							height="14"
						/>
						{t("sidebar.settings")}
					</button>
				</div>
			</CollapsibleSection>
		</div>

		<div
			class="user-profile"
			onclick={() => (showUserMenu = true)}
			role="button"
			tabindex="0"
			onkeydown={(e) =>
				(e.key === "Enter" || e.key === " ") && (showUserMenu = true)}
			style="cursor: pointer;"
		>
			<img
				src="https://minotar.net/avatar/{launcherStore.settings
					.username}"
				alt="Avatar"
				class="user-avatar"
			/>
			<div class="user-info">
				<div class="user-name-wrapper">
					<span class="user-name"
						>{launcherStore.settings.username}</span
					>
					<img
						src="/images/icons/edit.svg"
						alt={t("userMenu.edit")}
						class="user-edit-icon"
						width="12"
						height="12"
					/>
				</div>
				<span
					class="user-status"
					class:premium={launcherStore.settings.user}
				>
					{launcherStore.settings.user
						? t("userMenu.premium")
						: t("userMenu.offline")}
				</span>
			</div>
		</div>
	</div>
</aside>

<UserMenu bind:open={showUserMenu} />

<ModalBase bind:open={showRenameModal} title={t("sidebar.modals.editTitle")}>
	<div class="input-group" style="margin-top: 12px;">
		<label class="input-label" for="icon-selector"
			>{t("createInstance.iconLabel") || "Logo de la Instancia"}</label
		>
		<div id="icon-selector" class="icon-selector" style="margin-top: 8px;">
			{#each availableIcons as iconName (iconName)}
				{@const iconPath = `/images/instances/${iconName}`}
				<button
					type="button"
					class="icon-option"
					class:selected={selectedIcon === iconPath}
					onclick={() =>
						(selectedIcon =
							selectedIcon === iconPath ? null : iconPath)}
					title={iconName}
				>
					<img src={iconPath} alt={iconName} />
				</button>
			{/each}
		</div>
	</div>

	<div class="input-group">
		<label class="input-label" for="rename-input"
			>{t("sidebar.modals.nameLabel")}</label
		>
		<input
			id="rename-input"
			type="text"
			class="text-input"
			bind:value={renameInput}
			onkeydown={(e) => e.key === "Enter" && handleRename()}
		/>
	</div>

	<div class="input-group" style="margin-top: 12px;">
		<Select
			id="version-select"
			label={t("sidebar.modals.versionLabel")}
			options={versionOptions}
			bind:value={versionInput}
		/>
	</div>

	{#snippet footer()}
		<button
			type="button"
			class="btn-secondary"
			onclick={() => (showRenameModal = false)}
			>{t("sidebar.modals.cancel")}</button
		>
		<button type="button" class="btn-primary" onclick={handleRename}
			>{t("sidebar.modals.save")}</button
		>
	{/snippet}
</ModalBase>

<ModalBase bind:open={showDeleteModal} title={t("sidebar.modals.deleteTitle")}>
	<p
		style="font-size: 0.9rem; color: var(--text-secondary); line-height: 1.4;"
	>
		{t("sidebar.modals.deleteDesc1")}
		<strong style="color: var(--text-primary);"
			>"{instanceToActOn?.name}"</strong
		>{t("sidebar.modals.deleteDesc2")}
	</p>
	{#snippet footer()}
		<button
			type="button"
			class="btn-secondary"
			onclick={() => (showDeleteModal = false)}
			>{t("sidebar.modals.cancel")}</button
		>
		<button
			type="button"
			class="btn-primary"
			style="background: var(--color-error); color: white;"
			onclick={handleDelete}>{t("sidebar.modals.deleteBtn")}</button
		>
	{/snippet}
</ModalBase>

<style>
	.sidebar {
		width: var(--sidebar-width);
		flex-shrink: 0;
		background-color: var(--bg-sidebar);
		border-right: 1px solid var(--border);
		display: flex;
		flex-direction: column;
		padding: 18px 16px 12px;
		z-index: 10;
		user-select: none;
	}

	.sidebar-header {
		padding-bottom: 14px;
		margin-bottom: 8px;
		border-bottom: 1px solid var(--border);
	}

	.sidebar-header h1 {
		font-size: 0.65rem;
		font-weight: 700;
		letter-spacing: 2px;
		text-transform: uppercase;
		color: var(--text-secondary);
	}

	.section-label {
		font-size: 0.65rem;
		font-weight: 700;
		color: var(--text-secondary);
		text-transform: uppercase;
		letter-spacing: 1.5px;
		margin-bottom: 10px;
		display: block;
	}

	.sidebar-content {
		flex: 1;
		overflow-y: auto;
		min-height: 0;
		padding: 6px 0;
	}

	.instance-list {
		display: flex;
		flex-direction: column;
		gap: 3px;
	}

	.instance-item {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 8px 10px;
		border-radius: var(--border-radius-sm);
		cursor: pointer;
		transition:
			background 0.15s ease,
			border-color 0.15s ease;
		border: 1px solid transparent;
		background: transparent;
		color: var(--text-primary);
		width: 100%;
		text-align: left;
		font-family: "Cantarell", system-ui, sans-serif;
	}

	.instance-item:hover {
		background: rgba(255, 255, 255, 0.03);
	}

	.instance-item.active {
		background: var(--bg-item-active);
		border-color: var(--border);
	}

	.instance-icon {
		width: 22px;
		height: 22px;
		background: rgba(255, 255, 255, 0.04);
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		display: flex;
		align-items: center;
		justify-content: center;
		font-size: 0.7rem;
		flex-shrink: 0;
	}

	.instance-info-container {
		display: flex;
		align-items: center;
		gap: 10px;
		flex: 1;
		min-width: 0;
	}

	.instance-name {
		font-weight: 500;
		font-size: 0.85rem;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.instance-actions {
		display: flex;
		gap: 4px;
		opacity: 0;
		transition: opacity 0.2s ease;
	}

	.instance-item:hover .instance-actions {
		opacity: 1;
	}

	/* ── Section group (like Settings.svelte) ────────────────────────── */

	.sidebar-sections {
		margin-top: 6px;
		margin-bottom: -12px;
		border: 1px solid var(--border-color);
		border-radius: var(--border-radius-sm);
		overflow: hidden;
		width: calc(100% + 32px);
		margin-left: -16px;
		margin-right: -16px;
	}

	.sidebar-sections .sd-root {
		border: none;
		margin: 0;
		width: auto;
		border-bottom: 1px solid var(--border-color);
	}

	.sidebar-sections .section-full {
		margin: 0;
		width: auto;
	}

	.sidebar-sections .section-full :global(.cs-root) {
		background: transparent;
		border: none;
		border-bottom: 1px solid var(--border-color);
	}

	.sidebar-sections .user-profile {
		border: none;
		margin: 0;
		width: auto;
	}

	/* ── Downloads section ──────────────────────────────────────────── */

	.sd-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		width: 100%;
		background: none;
		border: none;
		color: inherit;
		padding: 8px 10px;
		cursor: pointer;
		border-radius: var(--border-radius-sm);
		transition: background 0.15s ease;
		user-select: none;
		font-family: "Cantarell", system-ui, sans-serif;
	}

	.sd-header:hover {
		background: rgba(255, 255, 255, 0.03);
	}

	.sd-header.expanded {
		border-bottom: 1px solid var(--border);
		border-radius: var(--border-radius-sm) var(--border-radius-sm) 0 0;
	}

	.sd-header-left {
		display: flex;
		align-items: center;
		gap: 7px;
		min-width: 0;
		flex: 1;
	}

	.sd-label {
		font-size: 0.75rem;
		font-weight: 700;
		color: var(--text-primary);
		text-transform: uppercase;
		letter-spacing: 0.05em;
		white-space: nowrap;
	}

	.sd-spinner {
		width: 10px;
		height: 10px;
		border: 1.5px solid var(--border);
		border-top-color: var(--text-muted);
		border-radius: 50%;
		animation: sd-spin 0.7s linear infinite;
		flex-shrink: 0;
	}

	.sd-spinner-sm {
		width: 8px;
		height: 8px;
		border: 1.5px solid var(--border);
		border-top-color: var(--text-muted);
		border-radius: 50%;
		animation: sd-spin 0.7s linear infinite;
		flex-shrink: 0;
		display: block;
	}

	@keyframes sd-spin {
		to {
			transform: rotate(360deg);
		}
	}

	.sd-chevron {
		color: var(--accent);
		flex-shrink: 0;
		transition: transform 0.2s;
	}

	.sd-chevron.open {
		transform: rotate(180deg);
	}

	.sd-body {
		overflow: hidden;
	}

	.sd-item {
		padding: 8px 10px;
		border-bottom: 1px solid var(--border);
		display: flex;
		flex-direction: column;
		gap: 6px;
	}

	.sd-item:last-child {
		border-bottom: none;
	}

	.sd-item-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
	}

	.sd-item-left {
		display: flex;
		align-items: center;
		gap: 6px;
		min-width: 0;
		flex: 1;
	}

	.sd-version {
		font-size: 0.72rem;
		font-weight: 700;
		color: var(--text-primary);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.sd-pct {
		font-size: 0.65rem;
		font-weight: 700;
		color: var(--text-muted);
		flex-shrink: 0;
	}

	.sd-pct.done {
		color: var(--color-success);
	}

	.sd-progress-track {
		width: 100%;
		height: 3px;
		background: var(--bg-input);
		border-radius: 2px;
		overflow: hidden;
	}

	.sd-progress-fill {
		height: 100%;
		background: var(--accent);
		border-radius: 2px;
		transition: width 0.35s cubic-bezier(0.4, 0, 0.2, 1);
	}

	.sd-progress-fill.done {
		background: var(--color-success);
	}

	.sd-empty {
		display: flex;
		align-items: center;
		justify-content: center;
		min-height: 130px;
		padding: 16px 10px;
		text-align: center;
		font-size: 0.68rem;
		color: var(--text-muted);
		line-height: 1.4;
	}

	/* ── Tools group ─────────────────────────────────────────────────── */

	:global(.tools-group) {
		display: flex;
		flex-direction: column;
		gap: 4px;
		width: 100%;
	}

	.tools-btn {
		background: transparent;
		border: 1px solid var(--border);
		color: var(--text-secondary);
		padding: 7px 10px;
		border-radius: var(--border-radius-sm);
		font-size: 0.75rem;
		font-weight: 500;
		cursor: pointer;
		display: flex;
		align-items: center;
		gap: 7px;
		width: 100%;
		transition:
			background 0.15s ease,
			color 0.15s ease;
		font-family: "Cantarell", system-ui, sans-serif;
	}

	.tools-btn:hover {
		background: var(--bg-item-active);
		color: var(--text-primary);
	}

	.tools-btn img {
		filter: var(--icon-filter);
		flex-shrink: 0;
	}

	/* ── User profile ────────────────────────────────────────────────── */

	.user-profile {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 10px;
		margin-top: auto;
		background: var(--bg-item-active);
	}

	.user-avatar {
		width: 28px;
		height: 28px;
		border-radius: var(--border-radius-sm);
		background: rgba(255, 255, 255, 0.04);
		border: 1px solid var(--border);
	}

	.user-info {
		display: flex;
		flex-direction: column;
		overflow: hidden;
		gap: 2px;
	}

	.user-name {
		font-size: 0.82rem;
		font-weight: 600;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.user-name-wrapper {
		display: flex;
		align-items: center;
		gap: 6px;
	}

	.user-edit-icon {
		opacity: 0;
		filter: invert(1);
		transition:
			opacity 0.2s ease,
			transform 0.2s ease;
		transform: translateX(-4px);
		pointer-events: none;
	}

	.user-profile:hover .user-edit-icon {
		opacity: 0.5;
		transform: translateX(0);
	}

	.user-status {
		font-size: 0.68rem;
		color: var(--text-secondary);
		letter-spacing: 0.3px;
		transition: color 0.2s ease;
	}

	.user-status.premium {
		color: var(--accent);
		font-weight: 600;
	}

	@media (max-width: 650px) {
		.sidebar {
			width: 70px;
			padding: 15px 10px;
		}

		.sidebar-header h1,
		.instance-name,
		.tools-btn,
		.sd-label,
		.sd-version,
		.sd-pct,
		.user-info {
			display: none;
		}

		.instance-item {
			justify-content: center;
			padding: 12px 0;
		}

		.sd-header {
			justify-content: center;
			padding: 8px 4px;
		}

		.sd-header .sd-chevron {
			display: none;
		}

		.sd-item {
			padding: 6px 4px;
			align-items: center;
		}

		.sd-item-header {
			justify-content: center;
		}

		.sd-item .sd-spinner-sm {
			width: 10px;
			height: 10px;
		}

		.user-profile {
			justify-content: center;
		}

		.sidebar-sections {
			margin-left: -10px;
			margin-right: -10px;
			width: calc(100% + 20px);
			margin-bottom: -15px;
		}
	}
</style>
