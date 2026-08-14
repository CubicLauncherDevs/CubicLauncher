<script lang="ts">
	import { t } from "$lib/i18n";
	import ModpackImportStep from "./ModpackImportStep.svelte";
	import InstanceImportStep from "./InstanceImportStep.svelte";

	let {
		initialMrpackPath = null,
		initialInstanceZipPath = null,
		name = $bindable(""),
		onImported,
	}: {
		initialMrpackPath?: string | null;
		initialInstanceZipPath?: string | null;
		name?: string;
		onImported?: () => void;
	} = $props();

	type SubTab = "modpack" | "instance";
	let subTab = $state<SubTab>("modpack");

	$effect(() => {
		if (initialMrpackPath) {
			subTab = "modpack";
		} else if (initialInstanceZipPath) {
			subTab = "instance";
		}
	});
</script>

<div class="local-import-step">
	<div class="sub-tab-bar" role="tablist">
		<button
			type="button"
			class="sub-tab-btn"
			role="tab"
			aria-selected={subTab === "modpack"}
			class:active={subTab === "modpack"}
			onclick={() => (subTab = "modpack")}
		>
			{t("createInstance.localModpackTab")}
		</button>
		<button
			type="button"
			class="sub-tab-btn"
			role="tab"
			aria-selected={subTab === "instance"}
			class:active={subTab === "instance"}
			onclick={() => (subTab = "instance")}
		>
			{t("createInstance.localInstanceTab")}
		</button>
	</div>

	<div class="sub-tab-panel" class:hidden={subTab !== "modpack"}>
		<ModpackImportStep
			{onImported}
			bind:name
			initialPath={initialMrpackPath}
		/>
	</div>

	<div class="sub-tab-panel" class:hidden={subTab !== "instance"}>
		<InstanceImportStep {onImported} initialPath={initialInstanceZipPath} />
	</div>
</div>

<style>
	.local-import-step {
		display: flex;
		flex-direction: column;
		gap: 12px;
		min-height: 280px;
	}

	.sub-tab-bar {
		display: flex;
		gap: 6px;
		border-bottom: 1px solid var(--border);
		padding-bottom: 8px;
	}

	.sub-tab-btn {
		padding: 5px 12px;
		border: 1px solid transparent;
		border-radius: var(--border-radius-sm);
		background: transparent;
		color: var(--text-secondary);
		font-size: 0.72rem;
		font-weight: 600;
		cursor: pointer;
		transition:
			color 0.15s ease,
			border-color 0.15s ease,
			background 0.15s ease;
	}

	.sub-tab-btn:hover {
		color: var(--text-primary);
		background: var(--bg-item-active);
	}

	.sub-tab-btn.active {
		color: var(--text-primary);
		border-color: var(--accent);
		background: rgba(var(--accent-rgb), 0.1);
	}

	.sub-tab-panel {
		flex: 1;
		min-height: 0;
	}

	.sub-tab-panel.hidden {
		display: none;
	}
</style>
