<script lang="ts">
	import { deleteInst, getActiveUser } from "$lib/api/launcherService";
	import { launcherStore } from "$lib/state/state.svelte";
	import { SvelteMap } from "svelte/reactivity";
	import type { InstanceDto } from "$lib/types/types";
	import UserMenu from "../UserMenu/UserMenu.svelte";
	import CollapsibleSection from "$lib/components/settings/CollapsibleSection.svelte";
	import DownloadQueue from "../DownloadQueue/DownloadQueue.svelte";
	import { t } from "$lib/i18n";
	import ContextMenu from "../ContextMenu.svelte";
	import { getVersions } from "$lib/api/launcherService";
	import InstanceItem from "./InstanceItem.svelte";
	import UserProfile from "./UserProfile.svelte";
	import DeleteInstanceModal from "./DeleteInstanceModal.svelte";

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
				{#if launcherStore.loadedInstances.length === 0}
					<div
						class="instance-item"
						style="opacity: 0.4; cursor: default;"
					>
						<span class="instance-name"
							>{t("sidebar.noInstances")}</span
						>
					</div>
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
]} />

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