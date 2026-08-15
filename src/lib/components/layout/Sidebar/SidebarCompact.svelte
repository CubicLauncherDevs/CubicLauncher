<script lang="ts">
	import { deleteInst, getActiveUser } from "$lib/api/launcherService";
	import { launcherStore } from "$lib/state/state.svelte";
	import {
		getAvatar,
		setAvatar,
		buildAvatarUrl,
	} from "$lib/state/avatarCache.svelte";
	import type { InstanceDto } from "$lib/types/types";
	import { t } from "$lib/i18n";
	import { tick } from "svelte";
	import type { Component, Snippet } from "svelte";
	import { getDisplayIconSrc } from "$lib/icons/logos";
	import DeleteInstanceModal from "./DeleteInstanceModal.svelte";
	import ChevronRightIcon from "$lib/icons/ChevronRightIcon.svelte";
	import CubicIcon from "$lib/icons/Cubic.svelte";

	interface Props {
		selectedInstance: InstanceDto | null;
		onopenquickmenu?: () => void;
		onopenprofileview?: () => void;
		onopeneditinstance: (instance: InstanceDto) => void;
		onopencreateinstance?: () => void;
		onopenversiondownloader?: () => void;
		onexpand?: () => void;
	}

	let {
		selectedInstance = $bindable(),
		onopenquickmenu,
		onopenprofileview,
		onopeneditinstance,
		onopencreateinstance,
		onopenversiondownloader,
		onexpand,
	}: Props = $props();

	let showDeleteModal = $state(false);
	let instanceToActOn = $state<InstanceDto | null>(null);
	let activeUser = $derived(getActiveUser());

	type SidebarContextMenuProps = {
		onedit: (instance: InstanceDto) => void;
		ondelete: (instance: InstanceDto) => void;
	};

	type SidebarContextMenuExports = {
		openContextMenu: (e: MouseEvent) => Promise<void>;
	};

	interface TooltipProps {
		open?: boolean;
		x?: number;
		y?: number;
		placement?: "right" | "left" | "top" | "bottom";
		children?: Snippet;
	}

	let ctxMenu = $state<SidebarContextMenuExports | undefined>();
	let ContextMenu = $state<Component<
		SidebarContextMenuProps,
		SidebarContextMenuExports
	> | null>(null);
	let Tooltip = $state<Component<TooltipProps, Record<string, never>> | null>(
		null,
	);
	let contextMenuLoadPromise: Promise<void> | null = null;
	let tooltipLoadPromise: Promise<void> | null = null;
	let loadingQueue: Promise<void> = Promise.resolve();

	let tooltipInstance = $state<InstanceDto | null>(null);
	let tooltipOpen = $state(false);
	let tooltipX = $state(0);
	let tooltipY = $state(0);

	function queueLoad(load: () => Promise<void>): Promise<void> {
		loadingQueue = loadingQueue.then(load).catch(() => {});
		return loadingQueue;
	}

	function startContextMenuImport(): Promise<void> {
		contextMenuLoadPromise = import("./SidebarContextMenu.svelte").then(
			(mod) => {
				ContextMenu = mod.default;
			},
		);
		return contextMenuLoadPromise;
	}

	function startTooltipImport(): Promise<void> {
		tooltipLoadPromise = import("$lib/components/ui/Tooltip.svelte").then(
			(mod) => {
				Tooltip = mod.default;
			},
		);
		return tooltipLoadPromise;
	}

	async function loadContextMenu() {
		return queueLoad(async () => {
			if (ContextMenu) return;
			if (contextMenuLoadPromise) {
				await contextMenuLoadPromise;
				return;
			}
			await startContextMenuImport();
		});
	}

	async function loadTooltip() {
		return queueLoad(async () => {
			if (Tooltip) return;
			if (tooltipLoadPromise) {
				await tooltipLoadPromise;
				return;
			}
			await startTooltipImport();
		});
	}

	function prefetchCompactOverlays() {
		if (typeof requestIdleCallback !== "undefined") {
			requestIdleCallback(() => {
				loadContextMenu().then(() => loadTooltip());
			});
		} else {
			setTimeout(() => {
				loadContextMenu().then(() => loadTooltip());
			}, 200);
		}
	}

	async function handleContextMenu(e: MouseEvent) {
		e.preventDefault();
		await loadContextMenu();

		const deadline = Date.now() + 100;
		while (!ctxMenu && Date.now() < deadline) {
			await tick();
		}

		ctxMenu?.openContextMenu(e);
	}

	async function showTooltip(
		e: MouseEvent | FocusEvent,
		instance: InstanceDto,
	) {
		const target = e.currentTarget as HTMLElement;
		const rect = target.getBoundingClientRect();
		tooltipX = rect.right;
		tooltipY = rect.top;
		tooltipInstance = instance;
		tooltipOpen = true;
		await loadTooltip();
	}

	function hideTooltip() {
		tooltipOpen = false;
	}

	prefetchCompactOverlays();

	let username = $derived(activeUser?.username ?? "Steve");

	let avatarSvg = $state("");

	$effect(() => {
		const user = activeUser;
		if (!user) return;

		const url = buildAvatarUrl(user.uuid, user.username, user.user_type);

		const cached = getAvatar(url);
		if (cached !== undefined) {
			avatarSvg = cached;
			return;
		}

		fetch(url)
			.then((r) => r.text())
			.then((svg) => {
				setAvatar(url, svg);
				avatarSvg = svg;
			})
			.catch(() => {});
	});

	function openDeleteModal(instance: InstanceDto) {
		instanceToActOn = instance;
		showDeleteModal = true;
	}

	async function handleDelete() {
		if (!instanceToActOn) return;
		const ok = await deleteInst(instanceToActOn.uuid);
		if (ok && selectedInstance?.uuid === instanceToActOn.uuid) {
			selectedInstance = null;
		}
		showDeleteModal = false;
	}
</script>

<aside class="sidebar-compact">
	<div class="sc-header">
		<CubicIcon width="20" height="20" />
	</div>

	<div class="sc-content">
		<div
			class="sc-instance-list"
			role="region"
			aria-label={t("sidebar.yourInstances")}
			data-tutorial="instance-list"
			oncontextmenu={handleContextMenu}
		>
			{#if launcherStore.loadedInstances.length === 0}
				<div class="sc-empty" title={t("sidebar.noInstances")}>—</div>
			{:else}
				{#each launcherStore.loadedInstances as instance (instance.uuid)}
					<button
						type="button"
						class="sc-instance-item"
						class:active={selectedInstance?.uuid === instance.uuid}
						data-instance-uuid={instance.uuid}
						onmouseenter={(e) => showTooltip(e, instance)}
						onmouseleave={hideTooltip}
						onfocus={(e) => showTooltip(e, instance)}
						onblur={hideTooltip}
						onclick={() =>
							(selectedInstance =
								selectedInstance?.uuid === instance.uuid
									? null
									: instance)}
					>
						<div class="sc-instance-icon">
							{#if instance.icon}
								<img
									src={getDisplayIconSrc(instance.icon)}
									alt={instance.name}
									width="18"
									height="18"
								/>
							{:else}
								{instance.name.charAt(0).toUpperCase()}
							{/if}
						</div>
					</button>
				{/each}
			{/if}
		</div>
	</div>

	<div class="sc-tools">
		<button
			type="button"
			class="sc-tool-btn"
			onclick={onopencreateinstance}
			title={t("sidebar.createInstance")}
		>
			<img
				src="/images/icons/nav/create.svg"
				alt={t("sidebar.createInstance")}
				width="16"
				height="16"
			/>
		</button>
		<button
			type="button"
			class="sc-tool-btn"
			onclick={onopenversiondownloader}
			title={t("sidebar.downloadVersions")}
		>
			<img
				src="/images/icons/ui/download.svg"
				alt={t("sidebar.downloadVersions")}
				width="16"
				height="16"
			/>
		</button>
		<button
			type="button"
			class="sc-tool-btn"
			onclick={onopenquickmenu}
			title={t("sidebar.settings")}
		>
			<img
				src="/images/icons/nav/settings.svg"
				alt={t("sidebar.settings")}
				width="16"
				height="16"
			/>
		</button>
	</div>

	<div
		class="sc-user"
		role="button"
		tabindex="0"
		title={username}
		onclick={() => onopenprofileview?.()}
		onkeydown={(e) =>
			(e.key === "Enter" || e.key === " ") && onopenprofileview?.()}
	>
		<div class="sc-avatar-wrapper">
			{#if avatarSvg}
				{@html avatarSvg}
			{/if}
		</div>
	</div>

	<button
		type="button"
		class="expand-btn"
		onclick={onexpand}
		title={t("sidebar.expand")}
	>
		<ChevronRightIcon size={14} />
	</button>
</aside>

<DeleteInstanceModal
	bind:open={showDeleteModal}
	instanceName={instanceToActOn?.name ?? ""}
	onconfirm={handleDelete}
/>

{#if ContextMenu}
	<ContextMenu
		bind:this={ctxMenu}
		onedit={(instance) => onopeneditinstance?.(instance)}
		ondelete={(instance) => openDeleteModal(instance)}
	/>
{/if}

{#if Tooltip}
	<Tooltip
		bind:open={tooltipOpen}
		x={tooltipX}
		y={tooltipY}
		placement="right"
	>
		{#if tooltipInstance}
			<div class="instance-tooltip">
				<span class="instance-tooltip-name">{tooltipInstance.name}</span
				>
				<span class="instance-tooltip-detail">
					{t("instanceView.details.version")}: {tooltipInstance.version}
				</span>
				<span class="instance-tooltip-detail">
					{t("instanceView.details.loader")}: {tooltipInstance.loader}
				</span>
			</div>
		{/if}
	</Tooltip>
{/if}

<style>
	.sidebar-compact {
		width: 70px;
		flex-shrink: 0;
		background: var(--bg-sidebar-gradient, var(--bg-sidebar));
		border-right: 1px solid var(--border);
		display: flex;
		flex-direction: column;
		padding: 14px 0 0;
		z-index: 10;
		user-select: none;
		align-items: center;
		position: relative;
	}

	.sc-header {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 10px;
		padding: 0 0 14px;
		margin-bottom: 8px;
		border-bottom: 1px solid var(--border);
		width: 100%;
	}

	.sc-header :global(.icon-svg) {
		color: var(--text-secondary);
	}

	.expand-btn {
		position: absolute;
		right: -12px;
		top: 50%;
		transform: translateY(-50%);
		z-index: 11;
		background: var(--bg-sidebar);
		border: 1px solid var(--border);
		color: var(--text-secondary);
		border-radius: var(--border-radius-sm);
		box-shadow: 0 0 6px rgba(0, 0, 0, 0.3);
		cursor: pointer;
		display: flex;
		align-items: center;
		justify-content: center;
		width: 24px;
		height: 28px;
		padding: 0;
		transition:
			background 0.15s ease,
			color 0.15s ease;
	}

	.expand-btn:hover {
		background: var(--bg-item-active);
		color: var(--text-primary);
	}

	.sc-content {
		flex: 1;
		overflow-y: auto;
		min-height: 0;
		width: 100%;
		padding: 4px 0;
	}

	.sc-instance-list {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 4px;
		padding: 0 8px;
	}

	.sc-empty {
		font-size: 0.75rem;
		color: var(--text-muted);
		padding: 10px 0;
	}

	.sc-instance-item {
		width: 36px;
		height: 36px;
		background: transparent;
		border: 1px solid transparent;
		border-radius: var(--border-radius-sm);
		cursor: pointer;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 0;
		transition:
			background 0.15s ease,
			border-color 0.15s ease;
	}

	.sc-instance-item:hover {
		background: var(--surface-selected);
	}

	.sc-instance-item.active {
		background: var(--bg-item-active);
		border-color: var(--border);
	}

	.sc-instance-icon {
		width: 28px;
		height: 28px;
		background: rgba(var(--surface-rgb), 0.04);
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		display: flex;
		align-items: center;
		justify-content: center;
		font-size: 0.7rem;
		flex-shrink: 0;
		color: var(--text-primary);
		overflow: hidden;
	}

	.sc-instance-icon img {
		width: 100%;
		height: 100%;
		object-fit: cover;
		display: block;
	}

	.sc-tools {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 6px;
		padding: 12px 0;
		margin: 0 8px;
		border-top: 1px solid var(--border);
	}

	.sc-tool-btn {
		width: 32px;
		height: 32px;
		background: transparent;
		border: 1px solid var(--border);
		color: var(--text-secondary);
		border-radius: var(--border-radius-sm);
		cursor: pointer;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 0;
		transition:
			background 0.15s ease,
			color 0.15s ease;
	}

	.sc-tool-btn:hover {
		background: var(--bg-item-active);
		color: var(--text-primary);
	}

	.sc-tool-btn img {
		filter: var(--icon-filter);
	}

	.sc-user {
		width: 100%;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 10px 0;
		margin-top: auto;
		background: var(--bg-item-active);
		cursor: pointer;
	}

	.sc-avatar-wrapper {
		width: 28px;
		height: 28px;
		border-radius: var(--border-radius-sm);
		border: 1px solid var(--border);
		flex-shrink: 0;
		background: var(--cubic-logo) center/60% no-repeat;
		overflow: hidden;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.sc-avatar-wrapper :global(svg) {
		width: 100%;
		height: 100%;
		display: block;
		border-radius: inherit;
	}

	.instance-tooltip {
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	.instance-tooltip-name {
		font-size: 0.82rem;
		font-weight: 700;
		color: var(--text-primary);
		white-space: nowrap;
	}

	.instance-tooltip-detail {
		font-size: 0.72rem;
		color: var(--text-secondary);
		white-space: nowrap;
	}
</style>
