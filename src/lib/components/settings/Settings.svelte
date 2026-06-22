<script lang="ts">
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import { listen } from "@tauri-apps/api/event";
	import { launcherStore } from "$lib/state/state.svelte";
	import { killInst, saveSettings } from "$lib/api/launcherService";
	import { openUrl } from "$lib/api/cubicApi";
	import { t } from "$lib/i18n";
	import Select from "$lib/components/layout/Select.svelte";
	import {
		checkForUpdates,
		downloadUpdate,
		installUpdate,
	} from "$lib/api/updaterServices";
	import { listThemes } from "$lib/api/themeManager";
	import {
		getJreVersions,
		installJre,
		uninstallJre,
	} from "$lib/api/cubicApi";
	import type { AppEvent, JreStatus, ThemeEntry } from "$lib/types/types";
	import UpdateSection from "./UpdateSection.svelte";
	import CollapsibleSection from "./CollapsibleSection.svelte";
	import JreCard from "./JreCard.svelte";
	import EnvVarEditor from "./EnvVarEditor.svelte";

	interface Props {
		onclose?: () => void;
	}

	let { onclose }: Props = $props();

	let saving = $state(false);
	let currentTab = $state("launcher");
	let checking = $state(false);
	let downloading = $state(false);
	let installing = $state(false);

	const JRE_VERSIONS = [8, 17, 21, 25];
	let jreStatuses = $state<Record<number, JreStatus>>({});
	let jreActionStates = $state<Record<number, string | undefined>>({});

	async function refreshJreStatus() {
		const statuses = await getJreVersions();
		const map: Record<number, JreStatus> = {};
		for (const s of statuses) {
			map[s.version] = s;
		}
		jreStatuses = map;
	}

	async function handleInstallJre(version: number) {
		if (jreActionStates[version]) return;
		jreActionStates[version] = "downloading";
		try {
			await installJre(version);
		} catch (e) {
			console.error(`Failed to queue JRE ${version}:`, e);
			jreActionStates[version] = undefined;
		}
	}

	async function handleUninstallJre(version: number) {
		if (jreActionStates[version]) return;
		jreActionStates[version] = "uninstalling";
		try {
			await uninstallJre(version);
		} catch (e) {
			console.error(`Failed to uninstall JRE ${version}:`, e);
		}
		jreActionStates[version] = undefined;
	}
	async function handleSave() {
		saving = true;
		await saveSettings();
		setTimeout(() => {
			saving = false;
		}, 1000);
	}

	async function autoDetectJava() {
		try {
			const paths: Record<string, string> =
				await invoke("detect_java_paths");
			for (const v of JRE_VERSIONS) {
				const p = paths[`jre${v}`];
				if (p)
					(
						launcherStore.settings as unknown as Record<
							string,
							string
						>
					)[`jre${v}_path`] = p;
			}
		} catch (e) {
			console.error("Failed to detect java paths", e);
		}
	}

	async function handleCheckForUpdates() {
		checking = true;
		await checkForUpdates(false);
		checking = false;
	}

	async function handleDownload() {
		downloading = true;
		await downloadUpdate();
		downloading = false;
	}

	async function handleInstall() {
		installing = true;
		await installUpdate();
		installing = false;
	}

	let tabs = $derived([
		{ id: "launcher", label: t("settings.tabs.launcher") },
		{ id: "minecraft", label: t("settings.tabs.minecraft") },
		{ id: "java", label: t("settings.tabs.java") },
	]);

	const languageOptions = [
		{ value: "es", label: "Español" },
		{ value: "en", label: "English" },
		{ value: "fr", label: "Français" },
		{ value: "de", label: "Deutsch" },
	];
	let availableThemes = $state<ThemeEntry[]>([]);
	let themeOptions = $derived(
		availableThemes.map((t: ThemeEntry) => ({
			value: t.id,
			label: t.name,
			badge: [t.author, t.version ? `v${t.version}` : ""]
				.filter(Boolean)
				.join(" "),
		})),
	);

	async function loadThemes() {
		availableThemes = await listThemes();
	}

	onMount(() => {
		loadThemes();
		refreshJreStatus();

		const unlisten = listen<AppEvent>("app-event", (event) => {
			if (event.payload.type === "DEnqueue") {
				const v = event.payload.data.version;
				const match = v.match(/^jre-(\d+)$/);
				if (match) {
					jreActionStates[Number(match[1])] = "downloading";
				}
			} else if (event.payload.type === "DFinish") {
				const v = event.payload.data.version;
				const match = v.match(/^jre-(\d+)$/);
				if (match) {
					jreActionStates[Number(match[1])] = undefined;
					refreshJreStatus();
				}
			} else if (event.payload.type === "JREChanged") {
				refreshJreStatus();
			}
		});

		return () => {
			unlisten.then((fn) => fn());
		};
	});
	let runningInstances = $derived(
		launcherStore.loadedInstances
			.filter((i) => i.status === "started" || i.status === "starting")
			.map((i) => i.uuid),
	);

	const currentVersion = __APP_VERSION__;

	function fmtVersion(v: string): string {
		if (!v) return v;
		const [major, , patch] = v.split(".");
		const patchNum = parseInt(patch ?? "0");
		return patchNum === 0 ? major : `${major} rev ${patchNum}`;
	}

	const displayVersion = $derived(fmtVersion(currentVersion));
	const formattedPendingUpdate = $derived(
		launcherStore.pendingUpdate
			? {
					version: fmtVersion(
						launcherStore.pendingUpdate.version ?? "",
					),
					body: launcherStore.pendingUpdate.body,
				}
			: null,
	);
</script>

<div class="qm-root">
	<!-- Header -->
	<div class="qm-header">
		<span class="qm-label">{t("settings.title")}</span>
		<button type="button" class="qm-close-btn" onclick={onclose}>✕</button>
	</div>

	<!-- Tab Navigation -->
	<div class="qm-tabs" data-tutorial="settings-tabs">
		{#each tabs as tab (tab)}
			<button
				type="button"
				class="qm-tab-btn"
				class:active={currentTab === tab.id}
				onclick={() => (currentTab = tab.id)}
				data-tutorial="tab-{tab.id}"
			>
				<span class="qm-tab-label">{tab.label}</span>
			</button>
		{/each}
	</div>

	<div class="qm-scroll" data-tutorial="settings-scroll">
		{#if currentTab === "launcher"}
			<div class="section-group">
				<CollapsibleSection
					title={t("settings.launcher.activeInstancesTitle")}
					iconSrc="/images/icons/play.svg"
					storageKey="section_instances"
				>
					{#each runningInstances as uuid (uuid)}
						{@const inst = launcherStore.loadedInstances.find(
							(i) => i.uuid === uuid,
						)}
						{#if inst}
							<div class="qm-active-card">
								<div class="qm-status-dot running"></div>
								<div class="qm-active-info">
									<span class="qm-active-name"
										>{inst.name}</span
									>
									<span class="qm-active-sub"
										>{inst.version} - {inst.loader}</span
									>
								</div>
								<button
									type="button"
									class="qm-kill-btn"
									onclick={() => killInst(inst.uuid)}
									>{t(
										"settings.launcher.killInstance",
									)}</button
								>
							</div>
						{/if}
					{:else}
						<div class="qm-empty-state">
							{t("settings.launcher.noInstances")}
						</div>
					{/each}
				</CollapsibleSection>

				<CollapsibleSection
					title={t("settings.launcher.updatesTitle")}
					iconSrc="/images/icons/download.svg"
					storageKey="section_updates"
				>
					<UpdateSection
						currentVersion={displayVersion}
						pendingUpdate={formattedPendingUpdate}
						updateProgress={launcherStore.updateProgress}
						updateDownloaded={launcherStore.updateDownloaded}
						{checking}
						{downloading}
						{installing}
						onCheck={handleCheckForUpdates}
						onDownload={handleDownload}
						onInstall={handleInstall}
					/>
				</CollapsibleSection>

				<CollapsibleSection
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
				</CollapsibleSection>

				<CollapsibleSection
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
							bind:checked={
								launcherStore.settings.discord_presence
							}
							onchange={handleSave}
						/>
						<label for="discord-presence"
							>{t("settings.launcher.discordPresence")}</label
						>
					</div>
				</CollapsibleSection>

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
							onclick={() =>
								openUrl("https://discord.gg/XQrRFWRyp")}
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
		{/if}

		{#if currentTab === "minecraft"}
			<div class="section-group">
				<CollapsibleSection
					title={t("settings.minecraft.perfTitle")}
					iconSrc="/images/icons/database.svg"
					storageKey="section_performance"
				>
					<div class="qm-field-group">
						<div class="qm-field">
							<label for="min-mem"
								>{t("settings.minecraft.minRam")}</label
							>
							<div class="qm-ram-stepper">
								<button
									type="button"
									class="qm-stepper-btn"
									onclick={() => {
										const v =
											launcherStore.settings.min_memory -
											0.5;
										if (v >= 0.5)
											launcherStore.settings.min_memory =
												v;
									}}>−</button
								>
								<span class="qm-ram-value"
									>{launcherStore.settings.min_memory} GB</span
								>
								<button
									type="button"
									class="qm-stepper-btn"
									onclick={() => {
										const v =
											launcherStore.settings.min_memory +
											0.5;
										if (
											v <=
											launcherStore.settings.max_memory
										)
											launcherStore.settings.min_memory =
												v;
									}}>+</button
								>
							</div>
						</div>
						<div class="qm-field">
							<label for="max-mem"
								>{t("settings.minecraft.maxRam")}</label
							>
							<div class="qm-ram-stepper">
								<button
									type="button"
									class="qm-stepper-btn"
									onclick={() => {
										const v =
											launcherStore.settings.max_memory -
											0.5;
										if (
											v >=
											launcherStore.settings.min_memory
										)
											launcherStore.settings.max_memory =
												v;
									}}>−</button
								>
								<span class="qm-ram-value"
									>{launcherStore.settings.max_memory} GB</span
								>
								<button
									type="button"
									class="qm-stepper-btn"
									onclick={() => {
										const v =
											launcherStore.settings.max_memory +
											0.5;
										if (v <= 64)
											launcherStore.settings.max_memory =
												v;
									}}>+</button
								>
							</div>
						</div>
					</div>
					<span class="qm-ram-hint"
						>{t("settings.minecraft.ramHint")}</span
					>
				</CollapsibleSection>

				<CollapsibleSection
					title={t("settings.minecraft.optionsTitle")}
					iconSrc="/images/icons/check-square.svg"
					storageKey="section_options"
				>
					<div class="qm-field-checkbox">
						<input
							type="checkbox"
							id="show-snapshots"
							bind:checked={launcherStore.settings.show_snapshots}
							onchange={handleSave}
						/>
						<label for="show-snapshots"
							>{t("settings.minecraft.showSnapshots")}</label
						>
					</div>
					<div class="qm-field-checkbox">
						<input
							type="checkbox"
							id="show-alpha"
							bind:checked={launcherStore.settings.show_alpha}
							onchange={handleSave}
						/>
						<label for="show-alpha"
							>{t("settings.minecraft.showAlpha")}</label
						>
					</div>
				</CollapsibleSection>
			</div>
		{/if}

		{#if currentTab === "java"}
			<div class="section-group">
				<CollapsibleSection
					title={t("settings.java.runtimesTitle")}
					iconSrc="/images/icons/terminal.svg"
					storageKey="section_runtimes"
				>
					<JreCard
						version={8}
						status={jreStatuses[8] ?? null}
						managed={launcherStore.settings.jre8_managed}
						path={launcherStore.settings.jre8_path}
						pathLabel={t("settings.java.java8Path")}
						isInstalling={jreActionStates[8] === "downloading"}
						isUninstalling={jreActionStates[8] === "uninstalling"}
						onToggleManaged={(v) =>
							(launcherStore.settings.jre8_managed = v)}
						onInstall={handleInstallJre}
						onUninstall={handleUninstallJre}
						onPathChange={(v) =>
							(launcherStore.settings.jre8_path = v)}
					/>
					<JreCard
						version={17}
						status={jreStatuses[17] ?? null}
						managed={launcherStore.settings.jre17_managed}
						path={launcherStore.settings.jre17_path}
						pathLabel={t("settings.java.java17Path")}
						isInstalling={jreActionStates[17] === "downloading"}
						isUninstalling={jreActionStates[17] === "uninstalling"}
						onToggleManaged={(v) =>
							(launcherStore.settings.jre17_managed = v)}
						onInstall={handleInstallJre}
						onUninstall={handleUninstallJre}
						onPathChange={(v) =>
							(launcherStore.settings.jre17_path = v)}
					/>
					<JreCard
						version={21}
						status={jreStatuses[21] ?? null}
						managed={launcherStore.settings.jre21_managed}
						path={launcherStore.settings.jre21_path}
						pathLabel={t("settings.java.java21Path")}
						isInstalling={jreActionStates[21] === "downloading"}
						isUninstalling={jreActionStates[21] === "uninstalling"}
						onToggleManaged={(v) =>
							(launcherStore.settings.jre21_managed = v)}
						onInstall={handleInstallJre}
						onUninstall={handleUninstallJre}
						onPathChange={(v) =>
							(launcherStore.settings.jre21_path = v)}
					/>
					<JreCard
						version={25}
						status={jreStatuses[25] ?? null}
						managed={launcherStore.settings.jre25_managed}
						path={launcherStore.settings.jre25_path}
						pathLabel={t("settings.java.java25Path")}
						isInstalling={jreActionStates[25] === "downloading"}
						isUninstalling={jreActionStates[25] === "uninstalling"}
						onToggleManaged={(v) =>
							(launcherStore.settings.jre25_managed = v)}
						onInstall={handleInstallJre}
						onUninstall={handleUninstallJre}
						onPathChange={(v) =>
							(launcherStore.settings.jre25_path = v)}
					/>
					<div style="margin-top: 12px;">
						<button
							type="button"
							class="detect-btn"
							onclick={autoDetectJava}
							>{t("settings.java.detectPathsBtn")}</button
						>
					</div>
					<div class="zulu-credit">
						{t("settings.java.zuluCredit")}
					</div>
					<span
						class="qm-themes-hint"
						onclick={() =>
							openUrl("https://www.cubiclauncher.com/docs/java")}
						role="link"
						tabindex="0"
						onkeydown={(e) => {
							if (e.key === "Enter")
								openUrl(
									"https://www.cubiclauncher.com/docs/java",
								);
						}}>{t("settings.launcher.javaSpan")}</span
					>
				</CollapsibleSection>

				<CollapsibleSection
					title={t("settings.advanced")}
					iconSrc="/images/icons/settings.svg"
					storageKey="section_advanced"
				>
					<div class="qm-field">
						<label for="jvm-args"
							>{t("settings.java.jvmArgs")}</label
						>
						<textarea
							id="jvm-args"
							bind:value={launcherStore.settings.jvm_args}
							placeholder="-Xmx2G -Xms1G ..."
							class="jvm-args-textarea"
						></textarea>
					</div>
					<EnvVarEditor
						initial={launcherStore.settings.env_vars}
						onchange={(vars) =>
							(launcherStore.settings.env_vars = vars)}
					/>
				</CollapsibleSection>
			</div>
		{/if}
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

	<!-- Footer -->
	<div class="qm-footer">
		<span class="qm-version">CubicLauncher v{displayVersion}</span>
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
