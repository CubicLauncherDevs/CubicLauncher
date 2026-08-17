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
	import Icon from "$lib/icons/Icon.svelte";
	import CollapsibleSection from "$lib/components/settings/CollapsibleSection.svelte";
	import DownloadQueue from "../DownloadQueue/DownloadQueue.svelte";
	import InstanceItem from "./InstanceItem.svelte";
	import UserProfile from "./UserProfile.svelte";
	import DeleteInstanceModal from "./DeleteInstanceModal.svelte";
	import SidebarContextMenu from "./SidebarContextMenu.svelte";

	interface Props {
		selectedInstance: InstanceDto | null;
		onopenquickmenu?: () => void;
		onopenprofileview?: () => void;
		onopeneditinstance: (instance: InstanceDto) => void;
		onopencreateinstance?: () => void;
		onopenversiondownloader?: () => void;
		oncollapse?: () => void;
	}

	let {
		selectedInstance = $bindable(),
		onopenquickmenu,
		onopenprofileview,
		onopeneditinstance,
		onopencreateinstance,
		onopenversiondownloader,
		oncollapse,
	}: Props = $props();

	let showDeleteModal = $state(false);
	let instanceToActOn = $state<InstanceDto | null>(null);
	let ctxMenu = $state<ReturnType<typeof SidebarContextMenu> | undefined>();

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

<aside class="sidebar">
	<div class="sidebar-header" data-tutorial="sidebar-header">
		<h1 style="font-size: 0.9rem; font-weight: bold;">CUBICLAUNCHER</h1>
	</div>

	<div class="sidebar-content">
		<div
			role="region"
			aria-label={t("sidebar.yourInstances")}
			oncontextmenu={(e) => ctxMenu?.openContextMenu(e)}
		>
			<span class="section-label">{t("sidebar.yourInstances")}</span>
			<div class="instance-list" data-tutorial="instance-list">
				{#if launcherStore.loadedInstances.length === 0}
					<div class="empty-instances">
						{t("sidebar.noInstances")}
					</div>
				{:else}
					{#each launcherStore.loadedInstances as instance (instance.uuid)}
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
				{/if}
			</div>
		</div>
	</div>

	<div class="sidebar-sections">
		<DownloadQueue />

		<div class="section-full">
			<CollapsibleSection
				title={t("sidebar.tools")}
				iconSrc="/images/icons/nav/sliders.svg"
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
							src="/images/icons/nav/create.svg"
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
							src="/images/icons/ui/download.svg"
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
							src="/images/icons/nav/settings.svg"
							alt=""
							width="14"
							height="14"
						/>
						{t("sidebar.settings")}
					</button>
				</div>
			</CollapsibleSection>
		</div>

		<div data-tutorial="user-profile">
			<UserProfile
				{username}
				{avatarSvg}
				{isPremium}
				{userTypeLabel}
				onclick={() => onopenprofileview?.()}
			/>
		</div>
	</div>

	<button
		type="button"
		class="collapse-btn"
		onclick={oncollapse}
		title={t("sidebar.collapse")}
	>
		<Icon src="/images/icons/ui/chevron-left.svg" size={14} />
	</button>
</aside>

<DeleteInstanceModal
	bind:open={showDeleteModal}
	instanceName={instanceToActOn?.name ?? ""}
	onconfirm={handleDelete}
/>

<SidebarContextMenu
	bind:this={ctxMenu}
	onedit={(instance) => onopeneditinstance?.(instance)}
	ondelete={(instance) => openDeleteModal(instance)}
/>

<style>
	.sidebar {
		width: var(--sidebar-width);
		flex-shrink: 0;
		background: var(--bg-sidebar-gradient, var(--bg-sidebar));
		border-right: 1px solid var(--border);
		display: flex;
		flex-direction: column;
		padding: 18px 16px 0;
		z-index: 10;
		user-select: none;
		position: relative;
	}

	.sidebar-header {
		padding-bottom: 14px;
		margin-bottom: 8px;
		border-bottom: 1px solid var(--border);
		display: flex;
		align-items: center;
		justify-content: space-between;
	}

	.sidebar-header h1 {
		font-size: 0.65rem;
		font-weight: 700;
		letter-spacing: 2px;
		text-transform: uppercase;
		color: var(--text-secondary);
	}

	.collapse-btn {
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

	.collapse-btn:hover {
		background: var(--bg-item-active);
		color: var(--text-primary);
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

	.empty-instances {
		padding: 10px 12px;
		font-size: 0.8rem;
		color: var(--text-muted);
		text-align: center;
	}

	.instance-list {
		display: flex;
		flex-direction: column;
		gap: 3px;
	}

	.sidebar-sections {
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
		}
	}
</style>
