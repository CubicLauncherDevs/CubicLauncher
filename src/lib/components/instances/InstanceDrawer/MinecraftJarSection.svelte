<script lang="ts">
	import { onMount, onDestroy } from "svelte";
	import { open } from "@tauri-apps/plugin-dialog";
	import { InstState, type InstanceDto } from "$lib/types/types";
	import { t } from "$lib/i18n";
	import {
		getMinecraftJar,
		minecraftJarAction,
		type MinecraftJarConfig,
		type JarAction,
	} from "$lib/api/minecraftJar";
	import ChevronDownIcon from "$lib/icons/ChevronDownIcon.svelte";
	import Trash from "$lib/icons/Trash.svelte";

	let { instance } = $props<{ instance: InstanceDto }>();
	let config = $state<MinecraftJarConfig | null>(null);
	let loading = $state(true);
	let working = $state(false);
	let error = $state("");
	let alive = true;
	const running = $derived(
		instance.status === InstState.Started ||
			instance.status === InstState.Starting,
	);
	const disabled = $derived(loading || working || running || !config);

	onMount(() => {
		void load();
	});
	onDestroy(() => {
		alive = false;
	});

	async function load() {
		loading = true;
		error = "";
		try {
			const result = await getMinecraftJar(instance.uuid);
			if (alive) config = result;
		} catch (err) {
			if (alive) error = String(err);
		} finally {
			if (alive) loading = false;
		}
	}

	async function perform(
		getAction: () => JarAction | Promise<JarAction | null>,
	) {
		if (disabled) return;
		working = true;
		error = "";
		const id = instance.uuid;
		try {
			const action = await getAction();
			if (!action || !alive) return;
			const result = await minecraftJarAction(id, action);
			if (alive) config = result;
		} catch (err) {
			if (alive) error = String(err);
		} finally {
			if (alive) working = false;
		}
	}

	function selectFiles(replace: boolean) {
		return perform(async () => {
			const selected = await open({
				title: t(replace ? "minecraftJar.replace" : "minecraftJar.add"),
				multiple: !replace,
				directory: false,
				filters: [
					{
						name: replace ? "JAR" : "JAR / ZIP",
						extensions: replace ? ["jar"] : ["jar", "zip"],
					},
				],
			});
			if (!selected) return null;
			const paths = Array.isArray(selected) ? selected : [selected];
			if (!paths.length) return null;
			return replace
				? { type: "replace", path: paths[0] }
				: { type: "add", paths };
		});
	}
</script>

<section
	class="jar-section"
	aria-label="Minecraft.jar"
	aria-busy={loading || working}
>
	<h3>Minecraft.jar</h3>
	<p class="hint">{t("minecraftJar.description")}</p>
	{#if running}<p class="hint">{t("minecraftJar.running")}</p>{/if}
	{#if loading}<p role="status">{t("minecraftJar.loading")}</p>{/if}
	{#if error}
		<p class="error" role="alert">{error}</p>
		{#if !config}<button type="button" onclick={load} disabled={loading}
				>{t("minecraftJar.retry")}</button
			>{/if}
	{/if}

	{#if config}
		<div class="current-row">
			<div class="current">
				<span class="label">{t("minecraftJar.current")}</span>
				<span class="badge">{config.replacement?.name ?? t("minecraftJar.original")}</span>
			</div>
			<button
				type="button"
				class="btn-danger restore-btn"
				disabled={disabled || !config?.replacement}
				onclick={() => perform(() => ({ type: "restore" }))}
			>
				{t("minecraftJar.restore")}
			</button>
		</div>
	{/if}

	<div class="action-bar">
		<button type="button" class="btn-primary" {disabled} onclick={() => selectFiles(false)}>
			{t("minecraftJar.add")}
		</button>
		<button type="button" class="btn-secondary" {disabled} onclick={() => selectFiles(true)}>
			{t("minecraftJar.replace")}
		</button>
	</div>

	{#if config}
		{#if config.mods.length}
			<h4 class="mods-title">Mods ({config.mods.length})</h4>
			<ol class="mods">
				{#each config.mods as mod, index (mod.file)}
					<li class="mod-row">
						<label class="mod-name">
							<input
								type="checkbox"
								checked={mod.enabled}
								{disabled}
								onchange={(event) => {
									const enabled = event.currentTarget.checked;
									event.currentTarget.checked = mod.enabled;
									void perform(() => ({
										type: "toggle",
										file: mod.file,
										enabled,
									}));
								}}
							/>
							<span>{mod.name}</span>
						</label>
						<div class="mod-actions">
							<button
								type="button"
								class="icon-btn"
								disabled={disabled || index === 0}
								aria-label={t("minecraftJar.moveUp", { name: mod.name })}
								title={t("minecraftJar.moveUp", { name: mod.name })}
								onclick={() =>
									perform(() => ({ type: "move", file: mod.file, offset: -1 }))
								}
							>
								<ChevronDownIcon class="chev rotate-180" size={16} />
							</button>
							<button
								type="button"
								class="icon-btn"
								disabled={disabled || index === config.mods.length - 1}
								aria-label={t("minecraftJar.moveDown", { name: mod.name })}
								title={t("minecraftJar.moveDown", { name: mod.name })}
								onclick={() =>
									perform(() => ({ type: "move", file: mod.file, offset: 1 }))
								}
							>
								<ChevronDownIcon class="chev" size={16} />
							</button>
							<button
								type="button"
								class="icon-btn"
								{disabled}
								aria-label={t("minecraftJar.removeNamed", { name: mod.name })}
								onclick={() =>
									perform(() => ({ type: "remove", file: mod.file }))
								}
								title={t("minecraftJar.remove")}
							>
								<Trash width="16" height="16" />
							</button>
						</div>
					</li>
				{/each}
			</ol>
		{:else}
			<p class="empty">{t("minecraftJar.empty")}</p>
		{/if}
	{/if}

	{#if working}<p role="status">{t("minecraftJar.working")}</p>{/if}
</section>

<style>
.jar-section {
		display: flex;
		flex-direction: column;
		gap: 12px;
		border-top: 1px solid var(--border-color);
		padding-top: 16px;
		margin-top: 16px;
	}
	h3,
	p {
		margin: 0;
	}
	h3 {
		font-size: 0.9rem;
	}
	p {
		font-size: 0.8rem;
		line-height: 1.5;
	}
	.hint {
		color: var(--text-muted);
	}
	.current,
	.error,
	.mod-name {
		overflow-wrap: anywhere;
	}
	.error {
		color: var(--danger, #ef4444);
	}
	.action-bar,
	.mod-actions {
		display: flex;
		flex-wrap: wrap;
		gap: 8px;
	}

	button {
		background: var(--bg-card);
		color: var(--text-primary);
		border: 1px solid var(--border-color);
		border-radius: var(--border-radius-sm);
		padding: 8px 10px;
		font: inherit;
		font-size: 0.8rem;
		cursor: pointer;
	}
	button:hover:not(:disabled) {
		background: var(--bg-item-active);
	}
	button:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.current-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 12px;
	}

	.current .label {
		font-weight: 700;
		margin-right: 8px;
		font-size: 0.8rem;
	}

	.badge {
		display: inline-block;
		padding: 2px 8px;
		border-radius: var(--border-radius-sm);
		background: var(--surface-selected);
		border: 1px solid var(--border-color);
		font-size: 0.75rem;
	}

	.restore-btn {
		padding: 6px 10px;
		font-size: 0.75rem;
	}

	.mods-title {
		margin: 8px 0 0;
		font-size: 0.85rem;
		color: var(--text-secondary);
	}
	.mods {
		margin: 0;
		padding-left: 0;
	}
	.mod-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 8px;
		padding: 8px 0;
		border-bottom: 1px solid var(--border-color);
	}
	.mod-name {
		display: flex;
		align-items: center;
		gap: 8px;
		font-size: 0.8rem;
		min-width: 0;
	}
	.mod-name input {
		flex-shrink: 0;
	}
	.mod-actions {
		gap: 4px;
	}

	.icon-btn {
		background: transparent;
		border: 1px solid var(--border-color);
		padding: 4px 6px;
		border-radius: var(--border-radius-sm);
		color: var(--text-secondary);
	}
	.icon-btn:hover:not(:disabled) {
		background: var(--surface-hover);
		color: var(--text-primary);
	}
	.chev {
		display: inline-block;
		color: currentColor;
	}
	.rotate-180 {
		transform: rotate(180deg);
	}

	.empty {
		color: var(--text-muted);
		font-size: 0.8rem;
	}


</style>
