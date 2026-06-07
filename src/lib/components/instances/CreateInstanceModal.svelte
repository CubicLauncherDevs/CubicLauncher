<script lang="ts">
	import { createInstance, getInstalledVersions } from "$lib/api/cubicApi";
	import { INSTANCE_LOGOS } from "$lib/icons/logos";
	import Select from "$lib/components/layout/Select.svelte";
	import ModalBase from "$lib/components/layout/ModalBase.svelte";
	import { t } from "$lib/i18n";

	let { open = $bindable(), oncreated } = $props<{
		open: boolean;
		oncreated?: () => void;
	}>();

	let name = $state("");
	let selectedVersion = $state("");
	let selectedIcon = $state<string | null>(null);
	let versions = $state<string[]>([]);
	let versionOptions = $derived(
		versions.map((v) => ({ value: v, label: v })),
	);
	let loading = $state(false);
	let error = $state<string | null>(null);

	$effect(() => {
		if (open) {
			fetchVersions();
		}
	});

	async function fetchVersions() {
		const rawVersions = await getInstalledVersions();
		versions = rawVersions.sort((a, b) =>
			b.localeCompare(a, undefined, {
				numeric: true,
				sensitivity: "base",
			}),
		);

		if (versions.length > 0 && !selectedVersion) {
			selectedVersion = versions[0];
		}
	}

	async function handleCreate() {
		if (!name.trim()) {
			error = t("createInstance.emptyNameErr");
			return;
		}

		if (!selectedVersion) {
			error = t("createInstance.noVersionsErr");
			return;
		}

		loading = true;
		error = null;

		try {
			await createInstance(
				name,
				selectedVersion,
				selectedIcon,
				() => {
					open = false;
					name = "";
					selectedIcon = null;
					oncreated?.();
				},
				(err: unknown) => {
					error = t("createInstance.createErr");
					console.error(err);
				},
			);
		} finally {
			loading = false;
		}
	}

	function reset() {
		open = false;
		name = "";
		selectedVersion = "";
		selectedIcon = null;
		versions = [];
		error = null;
	}
</script>

<ModalBase bind:open title={t("createInstance.title")} onclose={reset}>
	<div class="create-layout">
		<div class="create-logo-section">
			<span class="input-label">{t("createInstance.iconLabel")}</span>
			<div class="create-logo-preview">
				{#if selectedIcon}
					<img src={selectedIcon} alt="Logo" />
					<button
						type="button"
						class="create-logo-clear"
						onclick={() => (selectedIcon = null)}
						onmouseenter={(e) => (e.currentTarget.style.opacity = "1")}
						onmouseleave={(e) => (e.currentTarget.style.opacity = "0")}
					>Limpiar</button>
				{:else}
					<svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="color: var(--text-secondary); opacity: 0.5;">
						<rect x="3" y="3" width="18" height="18" rx="2" ry="2"></rect>
						<circle cx="8.5" cy="8.5" r="1.5"></circle>
						<polyline points="21 15 16 10 5 21"></polyline>
					</svg>
				{/if}
			</div>
		</div>

		<div class="create-details-section">
			<div class="input-group">
				<span class="input-label">{t("createInstance.nameLabel")}</span>
				<input
					type="text"
					class="text-input"
					bind:value={name}
					placeholder={t("createInstance.namePlaceholder")}
					disabled={loading}
					onkeydown={(e) => e.key === "Enter" && handleCreate()}
				/>
			</div>

			<div class="input-group">
				<Select
					label={t("createInstance.versionLabel")}
					bind:value={selectedVersion}
					options={versionOptions}
					disabled={loading || versions.length === 0}
					placeholder={t("createInstance.noVersionsErr")}
				/>
			</div>
		</div>
	</div>

	<div class="create-divider"></div>

	<div class="input-group">
		<span class="input-label">Seleccionar Icono</span>
		<div class="create-icon-grid">
			{#each INSTANCE_LOGOS as iconName (iconName)}
				{@const iconPath = `/images/instances/${iconName}`}
				<button
					type="button"
					class="icon-option"
					class:selected={selectedIcon === iconPath}
					onclick={() => (selectedIcon = selectedIcon === iconPath ? null : iconPath)}
					title={iconName}
				>
					<img src={iconPath} alt={iconName} />
				</button>
			{/each}
		</div>
	</div>

	{#if error}
		<div class="create-error">{error}</div>
	{/if}

	{#snippet footer()}
		<button
			type="button"
			class="btn-secondary"
			onclick={reset}
			disabled={loading}
		>
			{t("createInstance.cancel")}
		</button>
		<button
			type="button"
			class="btn-primary"
			onclick={handleCreate}
			disabled={loading}
		>
			{loading ? t("createInstance.creatingBtn") : t("createInstance.createBtn")}
		</button>
	{/snippet}
</ModalBase>

<style>
	.create-layout {
		display: flex;
		gap: 20px;
	}

	.create-logo-section {
		display: flex;
		flex-direction: column;
		gap: 12px;
		width: 100px;
		align-items: center;
		flex-shrink: 0;
	}

	.create-logo-preview {
		width: 80px;
		height: 80px;
		border-radius: 12px;
		background: rgba(255, 255, 255, 0.03);
		border: 2px dashed var(--border);
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 12px;
		position: relative;
		overflow: hidden;
	}

	.create-logo-preview img {
		width: 100%;
		height: 100%;
		object-fit: contain;
		filter: drop-shadow(0 4px 8px rgba(0, 0, 0, 0.3));
	}

	.create-logo-clear {
		position: absolute;
		inset: 0;
		background: rgba(0, 0, 0, 0.6);
		border: none;
		color: white;
		opacity: 0;
		cursor: pointer;
		transition: opacity 0.2s;
		font-size: 0.7rem;
		font-weight: bold;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.create-logo-clear:hover {
		opacity: 1;
	}

	.create-details-section {
		display: flex;
		flex-direction: column;
		gap: 16px;
		flex: 1;
	}

	.create-divider {
		height: 1px;
		background: var(--border);
		margin: 16px 0 12px;
	}

	.create-icon-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(44px, 1fr));
		gap: 8px;
		margin-top: 4px;
		max-height: 110px;
		overflow-y: auto;
		padding-right: 4px;
		padding-bottom: 4px;
	}

	.create-error {
		color: var(--color-error);
		font-size: 0.8rem;
		background: rgba(var(--color-error-rgb), 0.1);
		border: 1px solid rgba(var(--color-error-rgb), 0.2);
		border-radius: 6px;
		padding: 10px;
		text-align: center;
		font-weight: 500;
		margin-top: 8px;
	}
</style>
