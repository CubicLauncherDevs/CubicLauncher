<script lang="ts">
	import { t } from "#lib/i18n/index.js";
	import { animateHeight } from "#lib/utils/animateHeight.js";
	import { animDuration } from "#lib/utils/animations.js";
	import {
		downloads,
		getActiveDownloadCount,
		getDoneDownloadCount,
		getOverallPct,
		getStatusLabel,
	} from "#lib/state/downloadQueueState.svelte.js";
	import DownloadQueueHeader from "./DownloadQueueHeader.svelte";
	import DownloadQueueItem from "./DownloadQueueItem.svelte";

	let open = $state(false);
	let activeCount = $derived(getActiveDownloadCount());
	let doneCount = $derived(getDoneDownloadCount());
	const resizeDuration = $derived(animDuration(180));

	$effect(() => {
		if (activeCount > 0 && !open) {
			open = true;
		}
	});
</script>

<div class="sd-root" use:animateHeight={resizeDuration}>
	<div class="sd-content">
		<DownloadQueueHeader bind:open {activeCount} {doneCount} />
		{#if open}
			<div class="sd-body">
				{#if downloads.size === 0}
					<div class="sd-empty">{t("sidebar.noDownloadDesc")}</div>
				{:else}
					{#each [...downloads.values()] as item (item.version)}
						{@const overall = getOverallPct(item)}
						{@const label =
							!item.done && !item.error && item.activeType
								? getStatusLabel(item.activeType)
								: null}
						<DownloadQueueItem
							version={item.version}
							{overall}
							done={item.done}
							error={item.error}
							statusLabel={label}
						/>
					{/each}
				{/if}
			</div>
		{/if}
	</div>
</div>

<style>
	.sd-root {
		border-bottom: 1px solid var(--border-color);
	}

	.sd-body {
		overflow: hidden;
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
</style>
