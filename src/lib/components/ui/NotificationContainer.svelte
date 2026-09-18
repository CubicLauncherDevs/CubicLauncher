<script lang="ts">
	import { launcherStore } from "$lib/state/state.svelte";
	import NotificationToast from "./NotificationToast.svelte";
	import { notificationPreferences } from "./notificationPreferences";
	const preferences = $derived(
		notificationPreferences(launcherStore.settings),
	);
	const customized = $derived(preferences.enabled === true);
</script>

<div
	class="notification-container"
	class:customized
	data-position={customized ? preferences.position : undefined}
	data-size={customized ? preferences.size : undefined}
	style:--notification-title-size={customized
		? `${preferences.title_size / 14}rem`
		: null}
	style:--notification-message-size={customized
		? `${preferences.message_size / 14}rem`
		: null}
	style:--notification-title-case={!customized
		? null
		: preferences.uppercase_title
			? "uppercase"
			: "none"}
	style:--notification-title-weight={!customized
		? null
		: preferences.bold_title
			? "700"
			: "400"}
>
	{#each launcherStore.notifications as notification (notification.id)}
		<NotificationToast
			{notification}
			{customized}
			durationSeconds={preferences.duration_seconds}
		/>
	{/each}
</div>

<style>
	.notification-container {
		position: fixed;
		top: 1.5rem;
		right: 1.5rem;
		display: flex;
		flex-direction: column;
		align-items: flex-end;
		gap: 15px;
		z-index: 9999;
		pointer-events: none;
		max-width: calc(100vw - 3rem);
	}

	.notification-container.customized {
		align-items: stretch;
		gap: 8px;
		width: 344px;
		max-height: calc(100dvh - 3rem);
		box-sizing: border-box;
		padding: 8px;
		overflow-y: auto;
		overscroll-behavior: contain;
		--notification-offset-x: 24px;
		--notification-offset-y: 0px;
		--notification-padding: var(--toast-padding, 9px 16px 9px 9px);
		--notification-icon-size: 32px;
		--notification-body-gap: 2px;
		scrollbar-width: thin;
		scrollbar-color: var(--scrollbar-thumb) transparent;
	}

	.notification-container:empty {
		padding: 0;
	}

	.notification-container.customized:not(:empty) {
		pointer-events: auto;
	}

	.notification-container[data-size="normal"] {
		width: 424px;
		--notification-padding: var(
			--toast-prominent-padding,
			12px 18px 12px 12px
		);
	}

	.notification-container[data-size="wide"] {
		width: 504px;
		--notification-padding: var(
			--toast-prominent-padding,
			12px 18px 12px 12px
		);
	}

	.notification-container[data-position^="bottom"] {
		top: auto;
		bottom: 1.5rem;
		flex-direction: column-reverse;
	}

	.notification-container[data-position$="left"] {
		left: 1.5rem;
		right: auto;
		--notification-offset-x: -24px;
	}

	.notification-container[data-position$="center"] {
		left: 50%;
		right: auto;
		transform: translateX(-50%);
		--notification-offset-x: 0px;
		--notification-offset-y: -12px;
	}

	.notification-container[data-position="bottom-center"] {
		--notification-offset-y: 12px;
	}

	.notification-container :global(.notification-toast) {
		pointer-events: all;
	}
</style>
