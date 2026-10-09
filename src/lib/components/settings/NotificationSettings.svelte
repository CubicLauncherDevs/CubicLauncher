<script lang="ts">
	import { onDestroy } from "svelte";
	import { slide } from "svelte/transition";
	import {
		launcherStore,
		showInfo,
		removeNotification,
	} from "$lib/state/state.svelte";
	import { t } from "$lib/i18n";
	import { patchPreferences } from "$lib/utils/preferencePatch";
	import { animDuration } from "$lib/utils/animations";
	import NotificationPreview from "$lib/components/ui/NotificationPreview.svelte";
	import SegmentedControl from "./SegmentedControl.svelte";
	import "./controls.css";
	import {
		DEFAULT_NOTIFICATION_PREFERENCES,
		NOTIFICATION_POSITIONS,
		NOTIFICATION_SIZES,
		notificationPreferences,
		type NotificationPreferences,
	} from "$lib/components/ui/notificationPreferences";

	let { onsave }: { onsave: () => Promise<void> } = $props();
	const preferences = $derived(
		notificationPreferences(launcherStore.settings),
	);
	const sizes = $derived(
		NOTIFICATION_SIZES.map((value) => ({
			value,
			label: t(`settings.personalize.sizes.${value}`),
		})),
	);
	const ranges = [
		{ key: "title_size", min: 12, max: 24, unit: "px" },
		{ key: "message_size", min: 11, max: 22, unit: "px" },
		{ key: "duration_seconds", min: 3, max: 30, unit: "s" },
	] as const;
	let previewId: string | undefined;
	let dirty = false;
	const slideDuration = $derived(animDuration(150));

	function commit() {
		if (!dirty) return;
		dirty = false;
		void onsave();
	}

	function update(patch: Partial<NotificationPreferences>, save = true) {
		const current = launcherStore.settings.notification_preferences;
		const target = current ?? { ...preferences };
		if (!patchPreferences(target, patch)) return;
		if (!current) launcherStore.settings.notification_preferences = target;
		dirty = true;
		if (save) commit();
	}

	function preview() {
		if (previewId) removeNotification(previewId);
		previewId = showInfo(
			t("settings.launcher.testNotificationTitle"),
			t("settings.launcher.testNotificationMessage"),
		);
	}

	function reset() {
		update(DEFAULT_NOTIFICATION_PREFERENCES, false);
		if (launcherStore.settings.prominent_notifications) {
			launcherStore.settings.prominent_notifications = false;
			dirty = true;
		}
		commit();
	}

	onDestroy(() => {
		commit();
		if (previewId) removeNotification(previewId);
	});
</script>

{#snippet rangeControl(field: (typeof ranges)[number])}
	<div class="range-field">
		<label class="input-label" for="notification-{field.key}">
			{t(`settings.personalize.${field.key}`)}
			<output for="notification-{field.key}"
				>{preferences[field.key]} <span>{field.unit}</span></output
			>
		</label>
		<input
			class="qm-range-input"
			style:--range-fill={`${((preferences[field.key] - field.min) / (field.max - field.min)) * 100}%`}
			id="notification-{field.key}"
			type="range"
			min={field.min}
			max={field.max}
			step="1"
			value={preferences[field.key]}
			aria-describedby={field.key === "duration_seconds"
				? "notification-duration-hint"
				: undefined}
			oninput={(event) =>
				update(
					{ [field.key]: event.currentTarget.valueAsNumber },
					false,
				)}
			onchange={commit}
		/>
	</div>
{/snippet}

<div class="notification-settings settings-controls">
	<div class="enable-section">
		<div class="enable-switch">
			<input
				id="notification-customization"
				type="checkbox"
				role="switch"
				checked={preferences.enabled === true}
				aria-describedby="notification-customization-hint"
				onchange={(event) =>
					update({ enabled: event.currentTarget.checked })}
			/>
			<label for="notification-customization"
				>{t("settings.personalize.enabled")}</label
			>
		</div>
		<p id="notification-customization-hint" class="qm-ram-hint">
			{t("settings.personalize.enabledHint")}
		</p>
	</div>
	{#if preferences.enabled}
		<fieldset
			class="custom-options"
			aria-label={t("settings.personalize.notificationsTitle")}
			transition:slide={{ duration: slideDuration }}
		>
			<section
				class="option-group"
				aria-labelledby="notification-layout-heading"
			>
				<h3 id="notification-layout-heading">
					{t("settings.personalize.layoutTitle")}
				</h3>
				<div class="field">
					<span class="input-label" id="notification-position-label"
						>{t("settings.personalize.position")}</span
					>
					<div
						class="position-picker"
						role="group"
						aria-labelledby="notification-position-label"
					>
						{#each NOTIFICATION_POSITIONS as value (value)}
							<button
								type="button"
								class="pos-cell"
								data-pos={value}
								aria-pressed={preferences.position === value}
								aria-label={t(
									`settings.personalize.positions.${value}`,
								)}
								onclick={() => update({ position: value })}
							>
								<span class="pos-bar"></span>
							</button>
						{/each}
					</div>
				</div>
				<div class="field">
					<span class="input-label"
						>{t("settings.personalize.size")}</span
					>
					<SegmentedControl
						label={t("settings.personalize.size")}
						value={preferences.size}
						options={sizes}
						onchange={(value) =>
							update({
								size: value as NotificationPreferences["size"],
							})}
					/>
				</div>
			</section>
			<section
				class="option-group"
				aria-labelledby="notification-text-heading"
			>
				<h3 id="notification-text-heading">
					{t("settings.personalize.textTitle")}
				</h3>
				<div class="option-grid">
					{@render rangeControl(ranges[0])}
					{@render rangeControl(ranges[1])}
				</div>
				<div class="title-options">
					{#each ["uppercase_title", "bold_title"] as key (key)}
						<div class="qm-field-checkbox">
							<input
								id="notification-{key}"
								type="checkbox"
								checked={preferences[
									key as "uppercase_title" | "bold_title"
								]}
								onchange={(event) =>
									update({
										[key]: event.currentTarget.checked,
									})}
							/>
							<label for="notification-{key}"
								>{t(`settings.personalize.${key}`)}</label
							>
						</div>
					{/each}
				</div>
			</section>
			<section
				class="option-group"
				aria-labelledby="notification-timing-heading"
			>
				<h3 id="notification-timing-heading">
					{t("settings.personalize.behaviorTitle")}
				</h3>
				{@render rangeControl(ranges[2])}
				<p id="notification-duration-hint" class="qm-ram-hint">
					{t("settings.personalize.durationHint")}
				</p>
			</section>
			<section
				class="option-group"
				aria-labelledby="notification-preview-heading"
			>
				<h3 id="notification-preview-heading">
					{t("settings.personalize.previewTitle")}
				</h3>
				<NotificationPreview {preferences} />
			</section>
		</fieldset>
	{/if}
	<div class="actions">
		<button type="button" class="detect-btn preview-btn" onclick={preview}
			>{t("settings.launcher.testNotification")}</button
		>
		<button type="button" class="detect-btn" onclick={reset}
			>{t("settings.personalize.resetNotifications")}</button
		>
	</div>
</div>

<style>
	.notification-settings {
		display: flex;
		flex-direction: column;
		container-type: inline-size;
	}
	.enable-section {
		padding: 8px 0 12px;
	}
	.enable-switch {
		display: flex;
		align-items: center;
		gap: 10px;
		cursor: pointer;
		user-select: none;
	}
	.enable-switch label {
		font-size: 0.85rem;
		color: var(--text-secondary);
		cursor: pointer;
		transition: color 0.2s;
	}
	.enable-switch:hover label {
		color: var(--text-primary);
	}
	.enable-switch input[type="checkbox"] {
		appearance: none;
		-webkit-appearance: none;
		position: relative;
		flex-shrink: 0;
		box-sizing: border-box;
		width: 38px;
		height: 22px;
		margin: 0;
		border-radius: 999px;
		background: var(--bg-input);
		border: 1px solid var(--border-color);
		box-shadow: var(--shadow-inset);
		cursor: pointer;
		transition:
			background 0.2s,
			border-color 0.2s;
	}
	.enable-switch input[type="checkbox"]::after {
		content: "";
		position: absolute;
		top: 2px;
		left: 2px;
		width: 14px;
		height: 14px;
		border-radius: 50%;
		background: var(--text-muted);
		transition:
			transform 0.2s,
			background 0.2s;
	}
	.enable-switch input[type="checkbox"]:checked {
		background: var(--accent);
		border-color: var(--accent);
	}
	.enable-switch input[type="checkbox"]:checked::after {
		transform: translateX(16px);
		background: var(--accent-text);
	}
	.enable-switch input[type="checkbox"]:focus-visible {
		outline: 2px solid var(--accent);
		outline-offset: 3px;
	}
	.notification-settings :global(.qm-field-checkbox) {
		margin: 0;
		gap: 8px;
	}
	.notification-settings :global(.qm-ram-hint) {
		margin: 6px 0 0;
		padding: 0;
	}
	.enable-section .qm-ram-hint {
		padding-left: 48px;
	}
	.custom-options {
		min-width: 0;
		margin: 0;
		padding: 0;
		border: 0;
	}
	.option-group {
		padding: calc(11px * var(--cubic-interface-gap-factor, 1)) 0;
		border-top: 1px solid var(--border-color);
	}
	h3 {
		margin: 0 0 10px;
		font-size: 0.7rem;
		font-weight: 700;
		letter-spacing: 0.05em;
		text-transform: uppercase;
		color: var(--text-muted);
	}
	.field + .field {
		margin-top: 14px;
	}
	.position-picker {
		display: grid;
		grid-template-columns: repeat(3, minmax(0, 1fr));
		gap: 6px;
		margin-top: 8px;
	}
	.pos-cell {
		position: relative;
		display: flex;
		height: 34px;
		padding: 0;
		border: 1px solid var(--border-color);
		border-radius: var(--border-radius-sm);
		background: var(--bg-input);
		cursor: pointer;
		transition:
			border-color 0.15s,
			background 0.15s;
	}
	.pos-cell:hover {
		border-color: var(--text-muted);
	}
	.pos-cell[aria-pressed="true"] {
		border-color: var(--accent);
		background: rgba(var(--accent-rgb), 0.12);
	}
	.pos-cell[data-pos^="top"] {
		align-items: flex-start;
	}
	.pos-cell[data-pos^="bottom"] {
		align-items: flex-end;
	}
	.pos-cell[data-pos$="left"] {
		justify-content: flex-start;
	}
	.pos-cell[data-pos$="center"] {
		justify-content: center;
	}
	.pos-cell[data-pos$="right"] {
		justify-content: flex-end;
	}
	.pos-cell:focus-visible {
		outline: 2px solid var(--accent);
		outline-offset: 2px;
	}
	.pos-bar {
		width: 16px;
		height: 5px;
		margin: 5px;
		border-radius: 3px;
		background: var(--text-muted);
		transition: background 0.15s;
	}
	.pos-cell[aria-pressed="true"] .pos-bar {
		background: var(--accent);
	}
	.field :global(.segmented-control) {
		margin-top: 8px;
	}
	.option-grid {
		display: grid;
		grid-template-columns: minmax(0, 1fr);
		gap: calc(14px * var(--cubic-interface-gap-factor, 1));
	}
	.title-options {
		display: flex;
		flex-wrap: wrap;
		gap: 10px 18px;
		margin-top: 12px;
	}
	.title-options :global(label) {
		font-size: 0.8rem;
	}
	.range-field {
		display: flex;
		flex-direction: column;
		min-width: 0;
		gap: 6px;
	}
	.range-field label {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 8px;
	}
	output {
		flex-shrink: 0;
		padding: 2px 6px;
		border: 1px solid var(--border-color);
		border-radius: var(--border-radius-sm);
		background: var(--bg-input);
		font-size: 0.75rem;
		font-variant-numeric: tabular-nums;
		font-weight: 500;
		letter-spacing: normal;
		text-transform: none;
		color: var(--text-primary);
		white-space: nowrap;
	}
	output span {
		color: var(--text-muted);
	}
	.actions {
		display: flex;
		flex-wrap: wrap;
		gap: 8px;
		padding-top: 12px;
		border-top: 1px solid var(--border-color);
	}
	.actions .preview-btn {
		color: var(--accent);
		background: rgba(var(--accent-rgb), 0.08);
		border-color: rgba(var(--accent-rgb), 0.3);
	}
	.actions .preview-btn:hover {
		color: var(--accent);
		background: rgba(var(--accent-rgb), 0.15);
		border-color: var(--accent);
	}
	@container (min-width: 360px) {
		.option-grid {
			grid-template-columns: repeat(2, minmax(0, 1fr));
		}
	}
</style>
