import type { Feature } from "$/lib/features/types";
import { tr } from "$lib/utils/i18n";

export const WIDGET_FEATURES: Feature[] = [
	{
		id: "widget.mascot",
		get name() {
			return tr("mascotLib.featureMascotName");
		},
		get description() {
			return tr("mascotLib.featureMascotDesc");
		},
		icon: "User",
		defaultEnabled: false,
		version: "1.0",
		minAppVersion: "1.0",
	},

	{
		id: "widget.music",
		get name() {
			return tr("mascotLib.featureMusicName");
		},
		get description() {
			return tr("mascotLib.featureMusicDesc");
		},
		icon: "Music",
		defaultEnabled: false,
		version: "1.0",
		minAppVersion: "1.0",
	},
];

