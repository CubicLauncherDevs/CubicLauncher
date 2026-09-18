<script lang="ts">
	import Drawer from "$lib/components/layout/Drawer.svelte";
	import InterfaceSettings from "$lib/components/settings/InterfaceSettings.svelte";
	import NotificationSettings from "$lib/components/settings/NotificationSettings.svelte";
	import CollapsibleSection from "$lib/components/settings/CollapsibleSection.svelte";
	import VirtualList from "$lib/components/layout/VirtualList.svelte";
	import MarketGrid from "$lib/components/market/MarketGrid.svelte";
	import InstalledItem from "$lib/components/market/InstalledItem.svelte";
	import { launcherStore } from "$lib/state/state.svelte";
	import { applyInterfaceDensity } from "$lib/api/interfaceAppearance";
	import type { MarketProject } from "$lib/types/market";
	let { onsave }: { onsave: () => Promise<void> } = $props();
	const items: MarketProject[] = Array.from({ length: 100 }, (_, index) => ({
		id: String(index),
		title: `Long installed resource name ${index}`,
		source: "local",
		description: "A resource used to verify layout",
		author: "CubicLauncher",
		icon: null,
		downloadCount: 0,
	}));
	const noop = () => {};
	$effect(() =>
		applyInterfaceDensity(
			launcherStore.settings.interface_preferences.density,
		),
	);
</script>

<div class="list-host">
	<VirtualList
		{items}
		itemHeight={84}
		itemHeightVar="--market-installed-row-height"
		itemGap={{ variable: "--market-installed-row-gap", fallback: 6 }}
		keyFn={(item) => item.id}
	>
		{#snippet children(project)}
			<InstalledItem
				{project}
				checked={false}
				compact
				busy={false}
				canToggle={false}
				onCheck={noop}
				onOpen={noop}
				onToggle={noop}
				onDelete={noop}
			/>
		{/snippet}
	</VirtualList>
</div>
<div class="grid-host">
	<MarketGrid
		count={items.length}
		getItem={(index) => items[index]}
		busy={false}
		active
		onRangeNeeded={noop}
		onLoadMore={noop}
	>
		{#snippet children(project)}
			<InstalledItem
				{project}
				checked={false}
				compact={false}
				busy={false}
				canToggle={false}
				onCheck={noop}
				onOpen={noop}
				onToggle={noop}
				onDelete={noop}
			/>
		{/snippet}
	</MarketGrid>
</div>
<Drawer open direction="right">
	<div class="qm-root">
		<div class="qm-header">Settings</div>
		<div class="qm-scroll">
			<CollapsibleSection title="Interface"
				><InterfaceSettings {onsave} /></CollapsibleSection
			>
			<CollapsibleSection title="Notifications"
				><NotificationSettings {onsave} /></CollapsibleSection
			>
		</div>
		<div class="qm-footer">CubicLauncher</div>
	</div>
</Drawer>

<style>
	.list-host {
		width: 500px;
		max-width: 100%;
		height: 220px;
	}
	.grid-host {
		width: 600px;
		max-width: 100%;
		height: 250px;
	}
</style>
