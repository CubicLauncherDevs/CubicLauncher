<script lang="ts">
	import {
		dismissJreInstallPrompt,
		showInfo,
		setPendingJreLaunch,
		clearPendingJreLaunch,
	} from "$lib/state/state.svelte";
	import { installJre } from "$lib/api/cubicApi";
	import { launcherStore } from "$lib/state/state.svelte";
	import { t } from "$lib/i18n";
	import ModalBase from "$lib/components/layout/ModalBase.svelte";
	import { fly } from "svelte/transition";

	let prompt = $derived(launcherStore.jreInstallPrompt);

	function close() {
		dismissJreInstallPrompt();
	}

	async function handleInstall() {
		if (!prompt) return;
		const targetInstance = prompt.instance;
		const jreVersion = prompt.version;

		setPendingJreLaunch(jreVersion, targetInstance);
		dismissJreInstallPrompt();

		showInfo(
			t("settings.java.installing"),
			t("settings.java.installVersion", { version: String(jreVersion) }),
		);

		try {
			await installJre(jreVersion);
		} catch {
			clearPendingJreLaunch();
		}
	}
</script>

{#if prompt}
	<ModalBase
		open={true}
		onclose={close}
		title={t("settings.java.notInstalled")}
	>
		<p transition:fly={{ y: 8, duration: 200 }} class="jre-dialog-text">
			{t("launch.jreMissingBody", {
				version: String(prompt.version),
			})}
		</p>
		{#snippet footer()}
			<button
				type="button"
				class="qm-btn qm-btn-secondary"
				onclick={close}
			>
				{t("common.cancel")}
			</button>
			<button
				type="button"
				class="qm-btn qm-btn-primary"
				onclick={handleInstall}
			>
				{t("settings.java.install")}
			</button>
		{/snippet}
	</ModalBase>
{/if}

<style>
	.jre-dialog-text {
		margin: 0;
		font-size: 0.9rem;
		color: var(--text-secondary, #aaa);
		line-height: 1.5;
	}

	.qm-btn {
		padding: 8px 20px;
		border-radius: var(--border-radius-sm, 6px);
		font-size: 0.85rem;
		font-weight: 600;
		cursor: pointer;
		border: none;
		transition: opacity 0.15s;
	}

	.qm-btn:hover {
		opacity: 0.85;
	}

	.qm-btn-secondary {
		background: var(--bg-input, #2a2a3e);
		color: var(--text-primary, #ddd);
		border: 1px solid var(--border-color, #3a3a4e);
	}

	.qm-btn-primary {
		background: var(--accent, #4a7cf7);
		color: var(--accent-text, #fff);
	}
</style>
