<script lang="ts">
	import { t } from "$lib/i18n";
	import { openInstanceDir } from "$lib/api/cubicApi";
	import { launcherStore } from "$lib/state/state.svelte";
	import type { InstanceDto } from "$lib/types/types";
	import type { ContextMenuItem } from "$lib/components/layout/ContextMenu.svelte";
	import type ContextMenuComponent from "$lib/components/layout/ContextMenu.svelte";

	type ContextMenuConstructor = typeof ContextMenuComponent;

	interface Props {
		onedit: (instance: InstanceDto) => void;
		ondelete: (instance: InstanceDto) => void;
	}

	let { onedit, ondelete }: Props = $props();

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

	function buildInstanceMenu(instance: InstanceDto): ContextMenuItem[] {
		return [
			{
				label: t("sidebar.edit"),
				action: () => onedit(instance),
			},
			{
				label: t("sidebar.openFolder"),
				action: () => openInstanceDir(instance.uuid),
			},
			{ separator: true, label: "" },
			{
				label: t("sidebar.delete"),
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
	<MenuComponent
		bind:open={ctxOpen}
		x={ctxX}
		y={ctxY}
		items={ctxItems}
	/>
{/if}
