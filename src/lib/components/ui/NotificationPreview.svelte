<script lang="ts">
	import type { Notification } from "$lib/types/types";
	import { t } from "$lib/i18n";
	import NotificationToast from "./NotificationToast.svelte";
	import type { NotificationPreferences } from "./notificationPreferences";

	let { preferences }: { preferences: NotificationPreferences } = $props();

	const sample = $derived<Notification>({
		id: "notification-preview",
		type: "info",
		title: t("settings.launcher.testNotificationTitle"),
		message: t("settings.launcher.testNotificationMessage"),
	});
</script>

<div class="notification-preview" aria-hidden="true">
	<div class="preview-screen" data-position={preferences.position}>
		<div
			class="preview-toast"
			data-size={preferences.size}
			style:--notification-title-size={`${preferences.title_size / 14}rem`}
			style:--notification-message-size={`${
				preferences.message_size / 14
			}rem`}
			style:--notification-title-case={preferences.uppercase_title
				? "uppercase"
				: "none"}
			style:--notification-title-weight={preferences.bold_title
				? "700"
				: "400"}
		>
			<NotificationToast
				notification={sample}
				customized
				preview
				durationSeconds={preferences.duration_seconds}
			/>
		</div>
	</div>
	<p class="preview-caption">
		{t("settings.personalize.previewDuration", {
			seconds: preferences.duration_seconds,
		})}
	</p>
</div>

<style>
	.notification-preview {
		display: flex;
		flex-direction: column;
		gap: 6px;
	}
	.preview-screen {
		position: relative;
		contain: layout paint;
		height: 156px;
		border: 1px solid var(--border-color);
		border-radius: var(--border-radius-sm);
		background: var(--bg-input);
		box-shadow: var(--shadow-inset);
		overflow: hidden;
	}
	.preview-toast {
		position: absolute;
		width: 100%;
	}
	.preview-screen[data-position^="top"] .preview-toast {
		top: 10px;
	}
	.preview-screen[data-position^="bottom"] .preview-toast {
		bottom: 10px;
	}
	.preview-screen[data-position$="left"] .preview-toast {
		left: 10px;
	}
	.preview-screen[data-position$="right"] .preview-toast {
		right: 10px;
	}
	.preview-screen[data-position$="center"] .preview-toast {
		left: 50%;
		transform: translateX(-50%);
	}
	.preview-toast[data-size="compact"] {
		width: 64%;
	}
	.preview-toast[data-size="normal"] {
		width: 82%;
	}
	.preview-toast[data-size="wide"] {
		width: 94%;
	}
	.preview-caption {
		margin: 0;
		text-align: right;
		font-size: 0.72rem;
		color: var(--text-muted);
	}
</style>
