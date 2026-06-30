<script lang="ts">
	import { deleteInst, getActiveUser } from "$lib/api/launcherService";
	import { launcherStore } from "$lib/state/state.svelte";
	import { SvelteMap } from "svelte/reactivity";
	import type { InstanceDto } from "$lib/types/types";
	import UserMenu from "./UserMenu.svelte";
	import ModalBase from "./ModalBase.svelte";
	import CollapsibleSection from "$lib/components/settings/CollapsibleSection.svelte";
	import DownloadQueue from "./DownloadQueue.svelte";
	import { t } from "$lib/i18n";
	import Trash from "$lib/icons/Trash.svelte";

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
			? `https://bohrium-js.cubiclauncher.org/api/elyby/head/${username}`
			: `https://bohrium-js.cubiclauncher.org/api/mojang/head/${username}`;

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

	// ── Instance CRUD ─────────────────────────────────────────────────────

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
		<div class="section-label">{t("sidebar.yourInstances")}</div>
		<div class="instance-list" data-tutorial="instance-list">
			{#each launcherStore.loadedInstances as instance (instance.uuid)}
				<div
					class="instance-item"
					class:active={selectedInstance?.uuid === instance.uuid}
					onclick={() =>
						(selectedInstance =
							selectedInstance?.uuid === instance.uuid
								? null
								: instance)}
					onkeydown={(e) => {
						if (e.key === "Enter" || e.key === " ")
							selectedInstance =
								selectedInstance?.uuid === instance.uuid
									? null
									: instance;
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
								onopeneditinstance?.(instance);
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
							<Trash width="12" height="12" />
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

		<div
			class="user-profile"
			data-tutorial="user-profile"
			onclick={() => (showUserMenu = true)}
			role="button"
			tabindex="0"
			onkeydown={(e) =>
				(e.key === "Enter" || e.key === " ") && (showUserMenu = true)}
			style="cursor: pointer;"
		>
			<div class="user-avatar-wrapper">
				{#if avatarSvg}
					{@html avatarSvg}
				{/if}
			</div>
			<div class="user-info">
				<div class="user-name-wrapper">
					<span class="user-name">{username}</span>
				</div>
				<span class="user-status" class:premium={isPremium}>
					{userTypeLabel}
				</span>
			</div>
		</div>
	</div>
</aside>

<!-- Hacer capaz sea buena idea mover esto -->
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

<UserMenu bind:open={showUserMenu} />

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
		cursor: pointer;
		transition:
			background 0.15s ease,
			border-color 0.15s ease;
		border: 1px solid transparent;
		background: transparent;
		color: var(--text-primary);
		width: 100%;
		text-align: left;
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

	.sidebar-sections .section-full {
		margin: 0;
		width: auto;
	}

	.sidebar-sections .section-full :global(.cs-root) {
		background: transparent;
		border: none;
		border-bottom: 1px solid var(--border);
	}

	.sidebar-sections .user-profile {
		border: none;
		margin: 0;
		width: auto;
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

	.user-avatar-wrapper {
		width: 28px;
		height: 28px;
		border-radius: var(--border-radius-sm);
		border: 1px solid var(--border);
		flex-shrink: 0;
		background: url("/images/cubic.svg") center/60% no-repeat;
		overflow: hidden;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.user-avatar-wrapper :global(svg) {
		width: 100%;
		height: 100%;
		display: block;
		border-radius: inherit;
	}

	.user-profile {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 10px;
		margin-top: auto;
		background: var(--bg-item-active);
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
		.user-info {
			display: none;
		}

		.instance-item {
			justify-content: center;
			padding: 12px 0;
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
