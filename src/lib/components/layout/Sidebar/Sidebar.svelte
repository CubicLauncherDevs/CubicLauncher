<script lang="ts">
	import { deleteInst, getActiveUser } from "$lib/api/launcherService";
	import { launcherStore, getUntaggedInstances } from "$lib/state/state.svelte";
	import { SvelteMap } from "svelte/reactivity";
	import type { InstanceDto, TagDto } from "$lib/types/types";
	import UserMenu from "../UserMenu/UserMenu.svelte";
	import CollapsibleSection from "$lib/components/settings/CollapsibleSection.svelte";
	import DownloadQueue from "../DownloadQueue/DownloadQueue.svelte";
	import { t } from "$lib/i18n";
	import ContextMenu from "../ContextMenu.svelte";
	import { getVersions } from "$lib/api/launcherService";
	import InstanceItem from "./InstanceItem.svelte";
	import TagSection from "./TagSection.svelte";
	import UserProfile from "./UserProfile.svelte";
	import DeleteInstanceModal from "./DeleteInstanceModal.svelte";
	import TagManager from "$lib/components/settings/TagManager.svelte";

	interface Props {
		selectedInstance: InstanceDto | null;
		onopenquickmenu?: () => void;
		onopeneditinstance: (instance: InstanceDto) => void;
		onopenversiondownloader?: () => void;
		onopencreateinstance?: () => void;
	}

	let {
		selectedInstance = $bindable(),
		onopenquickmenu,
		onopenversiondownloader,
		onopeneditinstance,
		onopencreateinstance,
	}: Props = $props();

	let showUserMenu = $state(false);
	let ctxOpen = $state(false);
	let ctxX = $state(0);
	let ctxY = $state(0);
	let showDeleteModal = $state(false);
	let instanceToActOn = $state<InstanceDto | null>(null);
	let showTagManager = $state(false);
	let untaggedCollapsed = $state(false);
	let activeUser = $derived(getActiveUser());
	let username = $derived(activeUser?.username ?? "Steve");
	let isPremium = $derived(activeUser?.user_type === "Microsoft");
	let isYggdrasil = $derived(activeUser?.user_type === "Yggdrasil");
	let userTypeLabel = $derived(
		isPremium
			? t("userMenu.premium")
			: isYggdrasil
				? t("userMenu.authInjector")
				: t("userMenu.offline"),
	);

	const avatarCache = new SvelteMap<string, string>();

	let avatarSvg = $state("");

	$effect(() => {
		if (!username) return;
		const url = isYggdrasil
			? `https://skins.cubiclauncher.org/api/elyby/head/${username}`
			: `https://skins.cubiclauncher.org/api/mojang/head/${username}`;

		const cached = avatarCache.get(url);
		if (cached !== undefined) {
			avatarSvg = cached;
			return;
		}

		fetch(url)
			.then((r) => r.text())
			.then((svg) => {
				avatarCache.set(url, svg);
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
		await deleteInst(instanceToActOn.uuid);
		if (selectedInstance?.uuid === instanceToActOn.uuid) {
			selectedInstance = null;
		}
		showDeleteModal = false;
	}

	let instancesByTag = $derived(() => {
		const map = new Map<string, InstanceDto[]>();
		for (const tag of launcherStore.tags) {
			const instances = launcherStore.loadedInstances.filter((i) => i.tags.includes(tag.id));
			if (instances.length > 0) {
				map.set(tag.id, instances);
			}
		}
		return map;
	});

	let untaggedInstances = $derived(getUntaggedInstances());
</script>

<aside class="sidebar">
	<div class="sidebar-header" data-tutorial="sidebar-header">
		<h1 style="font-size: 0.9rem; font-weight: bold;">CUBICLAUNCHER</h1>
	</div>

	<div class="sidebar-content">
		<div
			class="instances-area"
			role="region"
			aria-label={t("sidebar.yourInstances")}
			oncontextmenu={(e) => {
				e.preventDefault();
				ctxX = e.clientX;
				ctxY = e.clientY;
				ctxOpen = true;
			}}
		>
			<div class="section-label">{t("sidebar.yourInstances")}</div>
			<div class="instance-list" data-tutorial="instance-list">
				{#if launcherStore.loadedInstances.length === 0}
					<div
						class="instance-item"
						style="opacity: 0.4; cursor: default;"
					>
						<span class="instance-name"
							>{t("sidebar.noInstances")}</span
						>
					</div>
				{:else}
					{#each launcherStore.tags as tag (tag.id)}
						{@const tagInstances = launcherStore.loadedInstances.filter((i) => i.tags.includes(tag.id))}
						{#if tagInstances.length > 0}
							<TagSection
								{tag}
								instances={tagInstances}
								{selectedInstance}
								selected={false}
								onselect={(inst) => (selectedInstance = inst)}
								onedit={(inst) => onopeneditinstance?.(inst)}
								ondelete={(inst) => openDeleteModal(inst)}
								onrename={() => {}}
								ondeleteTag={() => {}}
							/>
						{/if}
					{/each}
					{#if untaggedInstances.length > 0}
						<div class="untagged-section">
							<button
								type="button"
								class="untagged-header"
								class:expanded={!untaggedCollapsed}
								onclick={() => (untaggedCollapsed = !untaggedCollapsed)}
							>
								<span class="untagged-label">{t("sidebar.untagged")}</span>
								<span class="untagged-count">{untaggedInstances.length}</span>
								<span class="untagged-chevron" class:rotated={!untaggedCollapsed}>▸</span>
							</button>
							<div class="untagged-body" class:expanded={!untaggedCollapsed}>
								<div class="untagged-inner">
									{#each untaggedInstances as instance (instance.uuid)}
										<InstanceItem
											{instance}
											selected={selectedInstance?.uuid === instance.uuid}
											onselect={() =>
												(selectedInstance =
													selectedInstance?.uuid === instance.uuid
														? null
														: instance)}
											onedit={() => onopeneditinstance?.(instance)}
											ondelete={() => openDeleteModal(instance)}
										/>
									{/each}
								</div>
							</div>
						</div>
					{/if}
				{/if}
			</div>
		</div>
	</div>

	<div class="sidebar-sections">
		<DownloadQueue />

		<div class="section-full">
			<CollapsibleSection
				title={t("sidebar.tools")}
				iconSrc="/images/icons/sliders.svg"
				storageKey="sidebar-tools"
			>
				<div class="tools-group" data-tutorial="tools-group">
					<button
						type="button"
						class="tools-btn"
						onclick={onopencreateinstance}
						data-tutorial="create-instance"
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
						data-tutorial="download-versions"
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
						data-tutorial="settings"
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

		<UserProfile
			{username}
			{avatarSvg}
			{isPremium}
			{userTypeLabel}
			onclick={() => (showUserMenu = true)}
		/>
	</div>
</aside>

<DeleteInstanceModal
	bind:open={showDeleteModal}
	instanceName={instanceToActOn?.name ?? ""}
	onconfirm={handleDelete}
/>

<UserMenu bind:open={showUserMenu} />

<ContextMenu bind:open={ctxOpen} x={ctxX} y={ctxY} items={[
	{ label: t("sidebar.createInstance"), action: () => onopencreateinstance?.() },
	{ label: t("sidebar.refreshInstances"), action: () => getVersions() },
	{ label: t("tags.manage"), action: () => (showTagManager = true) },
]} />

<TagManager bind:open={showTagManager} onclose={() => (showTagManager = false)} />

<style>
	.sidebar {
		width: var(--sidebar-width);
		flex-shrink: 0;
		background: var(--bg-sidebar-gradient, var(--bg-sidebar));
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
		border: 1px solid transparent;
		background: transparent;
		color: var(--text-primary);
		width: 100%;
		text-align: left;
	}

	.instance-name {
		font-weight: 500;
		font-size: 0.85rem;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

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

	.sidebar-sections .section-full {
		margin: 0;
		width: auto;
	}

	.sidebar-sections .section-full :global(.cs-root) {
		background: transparent;
		border: none;
		border-bottom: 1px solid var(--border);
	}

	.untagged-section {
		margin-top: 4px;
	}

	.untagged-header {
		display: flex;
		align-items: center;
		gap: 7px;
		padding: 6px 10px;
		width: 100%;
		background: transparent;
		border: none;
		border-left: 3px solid var(--text-muted);
		border-radius: 0 var(--border-radius-sm) var(--border-radius-sm) 0;
		color: var(--text-secondary);
		font-size: 0.75rem;
		font-weight: 600;
		cursor: pointer;
		font-family: var(--font-family);
		text-align: left;
		transition: background 0.15s;
	}

	.untagged-header:hover {
		background: rgba(255, 255, 255, 0.04);
	}

	.untagged-label {
		flex: 1;
		font-size: 0.75rem;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.untagged-count {
		font-size: 0.65rem;
		opacity: 0.5;
		margin-left: auto;
	}

	.untagged-chevron {
		font-size: 0.6rem;
		transition: transform 0.2s;
		opacity: 0.4;
		line-height: 1;
	}

	.untagged-chevron.rotated {
		transform: rotate(90deg);
	}

	.untagged-body {
		display: grid;
		grid-template-rows: 0fr;
		transition: grid-template-rows 0.2s ease;
	}

	.untagged-body.expanded {
		grid-template-rows: 1fr;
	}

	.untagged-inner {
		display: flex;
		flex-direction: column;
		gap: 2px;
		overflow: hidden;
		border-left: 2px solid var(--border-color);
		margin-left: 15px;
		padding-left: 8px;
	}

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
	}

	.tools-btn:hover {
		background: var(--bg-item-active);
		color: var(--text-primary);
	}

	.tools-btn img {
		filter: var(--icon-filter);
		flex-shrink: 0;
	}

	@media (max-width: 650px) {
		.sidebar {
			width: 70px;
			padding: 15px 10px;
		}

		.sidebar-header h1,
		.tools-btn {
			display: none;
		}

		.sidebar-sections {
			margin-left: -10px;
			margin-right: -10px;
			width: calc(100% + 20px);
			margin-bottom: -15px;
		}
	}
</style>