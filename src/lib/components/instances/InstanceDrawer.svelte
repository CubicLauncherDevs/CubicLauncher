<script lang="ts">
	import { invoke } from "@tauri-apps/api/core";
	import { launcherStore } from "$lib/state/state.svelte";
	import { killInst, saveSettings } from "$lib/api/launcherService";
	import { openUrl } from "$lib/api/cubicApi";
	import CollapsibleSection from "$lib/components/settings/CollapsibleSection.svelte";
	import { t } from "$lib/i18n";
	import Select from "$lib/components/layout/Select.svelte";

	interface Props {
		onclose?: () => void;
	}

	let { onclose }: Props = $props();

	let saving = $state(false);

	async function handleSave() {
		saving = true;
		await saveSettings();
		setTimeout(() => {
			saving = false;
		}, 1000);
	}
</script>

<div class="qm-root">
	<!-- Header -->
	<div class="qm-header">
		<span class="qm-label">{t("settings.title")}</span>
		<button type="button" class="qm-close-btn" onclick={onclose}>✕</button>
	</div>

	<div class="qm-scroll" data-tutorial="settings-scroll">
		<div class="section-group">
			<!-- <CollapsibleSection
				title={t("settings.launcher.themes")}
				iconSrc="/images/icons/pencil.svg"
				storageKey="section_themes"
			>
				<Select
					id="theme"
					label={t("settings.launcher.themesActive")}
					options={themeOptions}
					bind:value={launcherStore.settings.theme}
					onchange={async () => {
						try {
							await invoke("set_theme", {
								id: launcherStore.settings.theme,
							});
						} catch (e) {
							console.error("Error setting theme:", e);
						}
					}}
				/>
				<span
					class="qm-themes-hint"
					onclick={() =>
						openUrl("https://www.cubiclauncher.com/themes")}
					role="link"
					tabindex="0"
					onkeydown={(e) => {
						if (e.key === "Enter")
							openUrl("https://www.cubiclauncher.com/themes");
					}}>{t("settings.launcher.themesSpan")}</span
				>
			</CollapsibleSection> -->

			<!-- <CollapsibleSection
				title={t("settings.launcher.generalTitle")}
				iconSrc="/images/icons/sliders.svg"
				storageKey="section_general"
			>
				<Select
					id="language"
					label={t("settings.launcher.language")}
					options={languageOptions}
					bind:value={launcherStore.settings.language}
					onchange={handleSave}
				/>
				<div class="qm-field-checkbox">
					<input
						type="checkbox"
						id="auto-updates"
						bind:checked={launcherStore.settings.auto_updates}
						onchange={handleSave}
					/>
					<label for="auto-updates"
						>{t("settings.launcher.autoUpdates")}</label
					>
				</div>
				<div class="qm-field-checkbox">
					<input
						type="checkbox"
						id="discord-presence"
						bind:checked={launcherStore.settings.discord_presence}
						onchange={handleSave}
					/>
					<label for="discord-presence"
						>{t("settings.launcher.discordPresence")}</label
					>
				</div>
			</CollapsibleSection> -->

			<CollapsibleSection
				title={t("settings.about.title")}
				iconSrc="/images/cubic.svg"
				storageKey="section_about"
			>
				<div class="about-content">
					<p class="about-desc">
						{t("settings.about.description")}
					</p>
					<div
						role="button"
						tabindex="0"
						onclick={() => openUrl("https://discord.gg/XQrRFWRyp")}
						onkeydown={(e) => {
							if (e.key === "Enter")
								openUrl("https://discord.gg/XQrRFWRyp");
						}}
					>
						<img
							src="/images/icons/discord.svg"
							alt="Discord"
							class="about-discord-icon"
							tabindex="-1    "
						/>
					</div>
					<p class="about-credit">
						{t("settings.about.creditMadeBy")}
						<button
							type="button"
							class="about-link"
							onclick={() =>
								openUrl("https://github.com/staff6773")}
						>
							Notstaff
						</button>
						{t("settings.about.creditAnd")}
						<button
							type="button"
							class="about-link"
							onclick={() =>
								openUrl("https://github.com/santiagolxx")}
						>
							Santiagolxx
						</button>
						{t("settings.about.creditSuffix")}
					</p>
				</div>
			</CollapsibleSection>
		</div>
	</div>

	<div class="save-footer">
		<button
			type="button"
			class="qm-save-btn"
			onclick={handleSave}
			disabled={saving}
		>
			{saving ? t("settings.java.saving") : t("settings.java.saveBtn")}
		</button>
	</div>
</div>

<style>
	.section-group {
		border: 1px solid var(--border-color);
		overflow: hidden;
		margin-bottom: 16px;
	}

	.section-group :global(.cs-root) {
		border: none;
		border-bottom: 1px solid var(--border-color);
	}

	.section-group :global(.cs-root:last-child) {
		border-bottom: none;
	}

	.qm-active-card {
		background: var(--bg-card);
		border-radius: var(--border-radius-sm);
		padding: 10px 12px;
		display: flex;
		align-items: center;
		gap: 12px;
		border: 1px solid var(--border-color);
		box-shadow:
			var(--shadow-sm),
			inset 0 1px 0 rgba(255, 255, 255, 0.03);
		margin-bottom: 6px;
	}

	.qm-status-dot {
		width: 8px;
		height: 8px;
		border-radius: 50%;
	}

	.qm-status-dot.running {
		background: var(--color-success);
		box-shadow: 0 0 10px rgba(var(--color-success-rgb), 0.4);
	}

	.qm-active-info {
		flex: 1;
		display: flex;
		flex-direction: column;
	}

	.qm-active-name {
		font-weight: 600;
		font-size: 0.9rem;
	}

	.qm-active-sub {
		font-size: 0.75rem;
		color: var(--text-secondary);
	}

	.qm-kill-btn {
		background: rgba(var(--color-error-rgb), 0.1);
		color: var(--color-error);
		border: 1px solid rgba(var(--color-error-rgb), 0.2);
		padding: 4px 10px;
		border-radius: var(--border-radius-sm);
		font-size: 0.75rem;
		font-weight: 600;
		cursor: pointer;
		transition: all 0.2s;
	}

	.qm-kill-btn:hover {
		background: var(--color-error);
		color: var(--accent-text);
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

	.qm-field-group {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 15px;
	}

	.qm-ram-stepper {
		display: flex;
		align-items: center;
		background: var(--bg-input);
		border: 1px solid var(--border-color);
		border-radius: var(--border-radius-sm);
		box-shadow: inset 0 1px 2px rgba(0, 0, 0, 0.25);
	}

	.qm-stepper-btn {
		background: none;
		border: none;
		color: var(--text-secondary);
		padding: 8px 14px;
		font-size: 1.1rem;
		font-weight: 700;
		cursor: pointer;
		transition: color 0.15s;
		line-height: 1;
	}

	.qm-stepper-btn:hover {
		color: var(--text-primary);
	}

	.qm-ram-value {
		flex: 1;
		text-align: center;
		font-size: 0.9rem;
		font-weight: 600;
		color: var(--text-primary);
		padding: 8px 4px;
		user-select: none;
	}

	.qm-ram-hint {
		display: block;
		margin-top: 10px;
		font-size: 0.75rem;
		color: var(--text-muted);
		line-height: 1.5;
		padding: 0 4px;
	}

	.qm-save-btn {
		width: 100%;
		background: var(--bg-card);
		color: var(--text-primary);
		border: 1px solid var(--border-color);
		padding: 10px 12px;
		border-radius: var(--border-radius-sm);
		font-family: var(--font-family);
		font-weight: 600;
		cursor: pointer;
		transition:
			background 0.15s,
			border-color 0.15s;
		box-shadow: var(--shadow-sm);
	}

	.qm-save-btn:hover:not(:disabled) {
		background: var(--bg-item-active);
		border-color: var(--border-color);
	}

	.qm-save-btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.save-footer {
		padding: 12px 20px;
		border-top: 1px solid var(--border-color);
	}

	.detect-btn {
		background: var(--bg-input);
		border: 1px solid var(--border-color);
		color: var(--text-secondary);
		padding: 6px 12px;
		border-radius: var(--border-radius-sm);
		font-size: 0.7rem;
		font-weight: 600;
		cursor: pointer;
		transition:
			color 0.15s,
			border-color 0.15s;
	}

	.detect-btn:hover {
		color: var(--text-primary);
		border-color: var(--text-muted);
	}

	.qm-field-checkbox {
		display: flex;
		align-items: center;
		gap: 12px;
		margin-bottom: 12px;
		margin-top: 8px;
		cursor: pointer;
		user-select: none;
	}

	.qm-field-checkbox input[type="checkbox"] {
		appearance: none;
		-webkit-appearance: none;
		width: 18px;
		height: 18px;
		background: var(--bg-input);
		border: 1px solid var(--border-color);
		border-radius: var(--border-radius-sm);
		cursor: pointer;
		position: relative;
		transition: all 0.2s;
	}

	.qm-field-checkbox input[type="checkbox"]:checked {
		background: var(--accent);
		border-color: var(--accent);
	}

	.qm-field-checkbox input[type="checkbox"]:checked::after {
		content: "✓";
		position: absolute;
		top: 50%;
		left: 50%;
		transform: translate(-50%, -50%);
		color: var(--accent-text);
		font-size: 11px;
		font-weight: 800;
	}

	.qm-field-checkbox label {
		font-size: 0.85rem;
		color: var(--text-secondary);
		cursor: pointer;
		transition: color 0.2s;
	}

	.qm-field-checkbox:hover label {
		color: var(--text-primary);
	}

	.jvm-args-textarea {
		width: 100%;
		background: var(--bg-input);
		border: 1px solid var(--border-color);
		color: var(--text-primary);
		padding: 8px 10px;
		border-radius: var(--border-radius-sm);
		font-size: 0.85rem;
		resize: vertical;
		min-height: 60px;
		font-family: monospace;
		box-shadow: inset 0 1px 2px rgba(0, 0, 0, 0.2);
		box-sizing: border-box;
	}

	.jvm-args-textarea:focus {
		outline: none;
		border-color: var(--text-muted);
	}

	.zulu-credit {
		margin-top: 12px;
		font-size: 0.7rem;
		color: var(--text-muted);
		text-align: center;
		opacity: 0.7;
	}

	.qm-field-checkbox input[type="checkbox"]:hover {
		border-color: var(--text-muted);
	}

	.qm-themes-hint {
		display: block;
		margin-top: 8px;
		font-size: 0.75rem;
		color: var(--text-secondary);
		line-height: 1.4;
		cursor: pointer;
		transition: color 0.2s;
	}

	.qm-themes-hint:hover {
		color: var(--text-primary);
	}

	.about-content {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 14px;
		padding: 8px 4px;
		text-align: center;
	}

	.about-desc {
		font-size: 0.8rem;
		color: var(--text-secondary);
		line-height: 1.5;
		max-width: 280px;
		margin: 0;
	}

	.about-discord-icon {
		width: 20px;
		height: 20px;
		cursor: pointer;
		opacity: 0.5;
		filter: var(--icon-filter);
		flex-shrink: 0;
		transition: opacity 0.15s;
	}

	.about-discord-icon:hover {
		opacity: 0.8;
	}

	.about-credit {
		font-size: 0.7rem;
		color: var(--text-muted);
		margin: 0;
		opacity: 0.7;
	}

	.about-link {
		background: none;
		border: none;
		color: var(--accent);
		font-size: inherit;
		font-family: inherit;
		padding: 0;
		cursor: pointer;
		display: inline;
		transition: opacity 0.15s;
	}

	.about-link:hover {
		opacity: 0.8;
		text-decoration: underline;
	}
</style>
