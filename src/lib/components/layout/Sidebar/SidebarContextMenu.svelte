<script lang="ts">
	import { t } from "$lib/i18n";
	import { exportInstanceZip, openInstanceDir } from "$lib/api/cubicApi";
	import {
		launcherStore,
		removeNotification,
		showErrorParsed,
		showInfo,
		showSuccess,
	} from "$lib/state/state.svelte";
	import type { InstanceDto } from "$lib/types/types";
	import type { ContextMenuItem } from "$lib/components/layout/ContextMenu.svelte";
	import type ContextMenuComponent from "$lib/components/layout/ContextMenu.svelte";
	import { save } from "@tauri-apps/plugin-dialog";

	type ContextMenuConstructor = typeof ContextMenuComponent;

	interface Props {
		onedit: (instance: InstanceDto) => void;
		ondelete: (instance: InstanceDto) => void;
		onpin: (instance: InstanceDto, pinned: boolean) => void;
	}

	let { onedit, ondelete, onpin }: Props = $props();

	let ctxOpen = $state(false);
	let ctxX = $state(0);
	let ctxY = $state(0);
	let ctxItems = $state<ContextMenuItem[]>([]);
	let MenuComponent = $state<ContextMenuConstructor | null>(null);
	let loadPromise: Promise<void> | null = null;

	async function loadMenuComponent() {
		if (MenuComponent) return;
		if (loadPromise) {
			await loadPromise;
			return;
		}

		loadPromise = import("$lib/components/layout/ContextMenu.svelte").then(
			(mod) => {
				MenuComponent = mod.default;
			},
		);
		await loadPromise;
	}

	async function handleExport(instance: InstanceDto) {
		try {
			const dest = await save({
				defaultPath: `${instance.name}.zip`,
				filters: [{ name: "ZIP", extensions: ["zip"] }],
			});
			if (!dest) return;

			const notificationId = showInfo(
				t("notifications.exportingTitle"),
				t("notifications.exportingMessage", { name: instance.name }),
			);

			const path = await exportInstanceZip(instance.uuid, dest);
			removeNotification(notificationId);

			if (path) {
				showSuccess(
					t("notifications.exportTitle"),
					t("notifications.exportSuccess", { path }),
				);
			}
		} catch (err) {
			showErrorParsed(err);
		}
	}

	function buildInstanceMenu(instance: InstanceDto): ContextMenuItem[] {
		return [
			{
				label: instance.pinned ? t("sidebar.unpin") : t("sidebar.pin"),
				icon: "/images/icons/ui/pin.svg",
				action: () => onpin(instance, !instance.pinned),
			},
			{
				label: t("sidebar.edit"),
				icon: "/images/icons/nav/edit.svg",
				action: () => onedit(instance),
			},
			{
				label: t("sidebar.openFolder"),
				icon: "/images/icons/instance/folder.svg",
				action: () => openInstanceDir(instance.uuid),
			},
			{
				label: t("sidebar.export"),
				icon: "/images/icons/ui/download.svg",
				action: () => void handleExport(instance),
			},
			{ separator: true, label: "" },
			{
				label: t("sidebar.delete"),
				icon: "/images/icons/ui/trash.svg",
				variant: "danger",
				action: () => ondelete(instance),
			},
		];
	}

	export async function openContextMenu(e: MouseEvent) {
		const target = e.target as HTMLElement;
		const instanceEl = target.closest<HTMLElement>("[data-instance-uuid]");

		if (!instanceEl) return;

		const uuid = instanceEl.dataset.instanceUuid;
		const instance = launcherStore.loadedInstances.find(
			(i) => i.uuid === uuid,
		);
		if (!instance) return;

		e.preventDefault();
		ctxItems = buildInstanceMenu(instance);
		ctxX = e.clientX;
		ctxY = e.clientY;

		await loadMenuComponent();
		ctxOpen = true;
	}
</script>

{#if MenuComponent}
	<MenuComponent bind:open={ctxOpen} x={ctxX} y={ctxY} items={ctxItems} />
{/if}
