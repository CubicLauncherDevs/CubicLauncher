<script lang="ts">
	import { onDestroy } from "svelte";
	import {
		launcherStore,
		showInfo,
		removeNotification,
	} from "$lib/state/state.svelte";
	import { t } from "$lib/i18n";
	import Select from "$lib/components/layout/Select.svelte";
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
	const positions = $derived(
		NOTIFICATION_POSITIONS.map((value) => ({
			value,
			label: t(`settings.personalize.positions.${value}`),
		})),
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

	function update(patch: Partial<NotificationPreferences>, save = true) {
		launcherStore.settings.notification_preferences = {
			...preferences,
			...patch,
		};
		if (save) void onsave();
	}

	function preview() {
		if (previewId) removeNotification(previewId);
		previewId = showInfo(
			t("settings.launcher.testNotificationTitle"),
			t("settings.launcher.testNotificationMessage"),
		);
	}

	function reset() {
		launcherStore.settings.notification_preferences = {
			...DEFAULT_NOTIFICATION_PREFERENCES,
		};
		launcherStore.settings.prominent_notifications = false;
		void onsave();
	}

	onDestroy(() => {
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
			onchange={onsave}
		/>
	</div>
{/snippet}

<div class="notification-settings settings-controls">
	<div class="enable-section">
		<div class="qm-field-checkbox">
			<input
				id="notification-customization"
				type="checkbox"
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
	<fieldset
		class="custom-options"
		disabled={!preferences.enabled}
		aria-label={t("settings.personalize.notificationsTitle")}
	>
		<section
			class="option-group"
			aria-labelledby="notification-layout-heading"
		>
			<h3 id="notification-layout-heading">
				{t("settings.personalize.layoutTitle")}
			</h3>
			<div class="option-grid">
				<Select
					id="notification-position"
					label={t("settings.personalize.position")}
					value={preferences.position}
					options={positions}
					disabled={!preferences.enabled}
					onchange={(value) =>
						update({
							position:
								value as NotificationPreferences["position"],
						})}
				/>
				<Select
					id="notification-size"
					label={t("settings.personalize.size")}
					value={preferences.size}
					options={sizes}
					disabled={!preferences.enabled}
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
								update({ [key]: event.currentTarget.checked })}
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
	</fieldset>
	<div class="actions">
		<button type="button" class="detect-btn preview-btn" onclick={preview}
			>{t("settings.launcher.testNotification")}</button
		>
		<button type="button" class="detect-btn" onclick={reset}
			>{t("settings.personalize.reset")}</button
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
		padding: 8px 0 14px;
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
		padding-left: 26px;
	}
	.custom-options {
		min-width: 0;
		margin: 0;
		padding: 0;
		border: 0;
	}
	.custom-options:disabled {
		opacity: var(--disabled-opacity, 0.5);
	}
	.option-group {
		padding: 14px 0;
		border-top: 1px solid var(--border-color);
	}
	h3 {
		margin: 0 0 12px;
		font-size: 0.7rem;
		font-weight: 700;
		letter-spacing: 0.05em;
		text-transform: uppercase;
		color: var(--text-muted);
	}
	.option-grid {
		display: grid;
		grid-template-columns: minmax(0, 1fr);
		gap: 14px;
	}
	.option-grid :global(.custom-select-container) {
		min-width: 0;
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
