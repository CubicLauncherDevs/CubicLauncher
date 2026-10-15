/** Load a panel on demand and retain it for subsequent openings. */
export function lazyComponent<T>(
	load: () => Promise<{ default: T }>,
	isOpen: () => boolean,
	onError: (error: unknown) => void,
) {
	let component = $state.raw<T | null>(null);

	$effect(() => {
		if (!isOpen() || component) return;
		let active = true;
		void load().then(
			(module) => {
				if (active) component = module.default;
			},
			(error: unknown) => {
				if (active) onError(error);
			},
		);
		return () => {
			active = false;
		};
	});

	return {
		get component() {
			return component;
		},
	};
}
