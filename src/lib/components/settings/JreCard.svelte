<script lang="ts">
	import type { JreStatus } from "$lib/types/types";
	import { t } from "$lib/i18n";
	import Select from "$lib/components/layout/Select.svelte";

	interface Props {
		version: number;
		status: JreStatus | null;
		managed: boolean;
		path: string;
		pathLabel: string;
		isInstalling: boolean;
		isUninstalling: boolean;
		/** Proveedores con build descargable para esta versión de Java. */
		vendors: string[];
		/** Proveedor elegido; `auto` usa la cadena por defecto. */
		vendor: string;
		onToggleManaged: (managed: boolean) => void;
		onInstall: (version: number) => void;
		onUninstall: (version: number) => void;
		onPathChange: (path: string) => void;
		onVendorChange: (vendor: string) => void;
	}

	let {
		version,
		status,
		managed,
		path,
		pathLabel,
		isInstalling,
		isUninstalling,
		vendors,
		vendor,
		onToggleManaged,
		onInstall,
		onUninstall,
		onPathChange,
		onVendorChange,
	}: Props = $props();

	function vendorLabel(id: string): string {
		const key = `settings.java.vendors.${id}`;
		const label = t(key);
		return label === key ? id : label;
	}

	function vendorHint(id: string): string | undefined {
		const key = `settings.java.vendors.${id}Hint`;
		const hint = t(key);
		return hint === key ? undefined : hint;
	}

	/** El runtime instalado pertenece a otro proveedor del elegido. */
	const vendorMismatch = $derived(
		status?.installed === true &&
			vendor !== "auto" &&
			!!status.vendor &&
			vendor !== status.vendor,
	);

	const vendorOptions = $derived([
		{
			value: "auto",
			label: t("settings.java.providerAuto"),
			subtitle: t("settings.java.providerAutoHint"),
		},
		...vendors.map((id) => ({
			value: id,
			label: vendorLabel(id),
			subtitle: vendorHint(id),
		})),
	]);
</script>

<div class="qm-jre-card">
	<div class="qm-jre-header">
		<span class="qm-jre-title">Java {version}</span>
		<div class="qm-jre-toggle">
			<button
				type="button"
				class="qm-toggle-btn"
				class:active={managed}
				onclick={() => onToggleManaged(true)}
			>
				{t("settings.java.managed")}
			</button>
			<button
				type="button"
				class="qm-toggle-btn"
				class:active={!managed}
				onclick={() => onToggleManaged(false)}
			>
				{t("settings.java.external")}
			</button>
		</div>
	</div>
	{#if managed}
		<div class="qm-jre-managed">
			<div class="qm-jre-provider">
				<Select
					id={`jre${version}-vendor`}
					compact
					label={t("settings.java.provider")}
					value={vendor}
					options={vendorOptions}
					onchange={(value) => onVendorChange(value)}
				/>
			</div>
			{#if status}
				{#if status.installed}
					<div class="qm-jre-installed">
						<span class="qm-jre-version"
							>{t("settings.java.installedVersion", {
								version: status.java_version ?? "?",
							})}{#if status.vendor}
								· {t("settings.java.installedVendor", {
									vendor: vendorLabel(status.vendor),
								})}{/if}</span
						>
						<button
							type="button"
							class="qm-uninstall-btn"
							class:loading={isUninstalling}
							onclick={() => onUninstall(version)}
							disabled={isUninstalling}
						>
							{#if isUninstalling}
								<span class="qm-spinner"></span>
							{/if}
							{isUninstalling
								? t("settings.java.uninstalling")
								: t("settings.java.uninstall")}
						</button>
					</div>
					{#if vendorMismatch}
						<span class="qm-jre-hint"
							>{t("settings.java.providerMismatch", {
								vendor: vendorLabel(vendor),
							})}</span
						>
					{/if}
				{:else}
					<div class="qm-jre-not-installed">
						<span>{t("settings.java.notInstalled")}</span>
						<button
							type="button"
							class="qm-install-btn"
							class:loading={isInstalling}
							onclick={() => onInstall(version)}
							disabled={isInstalling}
						>
							{#if isInstalling}
								<span class="qm-spinner"></span>
							{/if}
							{isInstalling
								? t("settings.java.installing")
								: t("settings.java.install")}
						</button>
					</div>
				{/if}
			{:else}
				<span class="qm-jre-loading">...</span>
			{/if}
		</div>
	{:else}
		<div class="qm-jre-external">
			<div class="qm-field">
				<label for="jre{version}">{pathLabel}</label>
				<input
					type="text"
					id="jre{version}"
					value={path}
					oninput={(e) => onPathChange(e.currentTarget.value)}
					placeholder="Path to java"
				/>
			</div>
		</div>
	{/if}
</div>

<style>
	.qm-jre-card {
		background: var(--bg-card, #1a1a2e);
		border: 1px solid var(--border-color, #2a2a3e);
		border-radius: var(--border-radius-sm);
		padding: 12px;
		margin-bottom: 10px;
		box-shadow: var(--shadow-sm, 0 1px 3px rgba(0, 0, 0, 0.3));
		min-height: 60px;
	}

	.qm-jre-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-bottom: 10px;
	}

	.qm-jre-title {
		font-weight: 600;
		font-size: 0.9rem;
	}

	.qm-jre-toggle {
		display: flex;
		gap: 4px;
	}

	.qm-toggle-btn {
		background: var(--bg-input);
		border: 1px solid var(--border-color);
		color: var(--text-secondary);
		padding: 4px 10px;
		border-radius: var(--border-radius-sm);
		font-size: 0.7rem;
		font-weight: 600;
		cursor: pointer;
		transition: all 0.15s;
	}

	.qm-toggle-btn.active {
		background: var(--accent);
		color: var(--accent-text);
		border-color: var(--accent);
	}

	.qm-jre-managed {
		padding-top: 4px;
	}

	.qm-jre-provider {
		margin-bottom: 10px;
	}

	.qm-jre-hint {
		display: block;
		margin-top: 6px;
		font-size: 0.75rem;
		color: var(--text-muted);
	}

	.qm-jre-installed,
	.qm-jre-not-installed {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 8px;
	}

	.qm-jre-version {
		font-size: 0.8rem;
		color: var(--color-success);
	}

	.qm-jre-loading {
		font-size: 0.8rem;
		color: var(--text-muted);
	}

	.qm-install-btn {
		background: var(--accent);
		color: var(--accent-text);
		border: none;
		padding: 5px 14px;
		border-radius: var(--border-radius-sm);
		font-size: 0.75rem;
		font-weight: 600;
		cursor: pointer;
		transition: opacity 0.15s;
		display: inline-flex;
		align-items: center;
		gap: 6px;
	}

	.qm-install-btn:hover:not(:disabled) {
		opacity: 0.85;
	}

	.qm-install-btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.qm-uninstall-btn {
		background: rgba(var(--color-error-rgb), 0.1);
		color: var(--color-error);
		border: 1px solid rgba(var(--color-error-rgb), 0.2);
		padding: 4px 12px;
		border-radius: var(--border-radius-sm);
		font-size: 0.75rem;
		font-weight: 600;
		cursor: pointer;
		transition: all 0.15s;
		display: inline-flex;
		align-items: center;
		gap: 6px;
	}

	.qm-uninstall-btn:hover:not(:disabled) {
		background: var(--color-error);
		color: var(--accent-text);
	}

	.qm-uninstall-btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.qm-jre-external {
		padding-top: 4px;
	}

	.qm-jre-external .qm-field {
		margin-bottom: 0;
	}

	.qm-field {
		margin-bottom: 15px;
	}

	.qm-field label {
		display: block;
		font-size: 0.8rem;
		color: var(--text-secondary);
		margin-bottom: 6px;
	}

	.qm-field input {
		width: 100%;
		background: var(--bg-input);
		border: 1px solid var(--border-color);
		color: var(--text-primary);
		padding: 8px 10px;
		border-radius: var(--border-radius-sm);
		font-size: 0.85rem;
		transition: border-color 0.2s;
		box-sizing: border-box;
		box-shadow: var(--shadow-inset);
	}

	.qm-field input:focus {
		outline: none;
		border-color: var(--text-muted);
	}

	.qm-spinner {
		display: inline-block;
		width: 12px;
		height: 12px;
		border: 2px solid currentColor;
		border-right-color: transparent;
		border-radius: 50%;
		animation: qm-spin 0.6s linear infinite;
		will-change: transform;
		flex-shrink: 0;
	}

	@keyframes qm-spin {
		to {
			transform: rotate(360deg);
		}
	}
</style>
