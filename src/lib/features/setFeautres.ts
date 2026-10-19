import type { Feature } from "$/lib/features/types";

export const WIDGET_FEATURES: Feature[] = [
	{
		id: "widget.mascot",
		name: "캐릭터",
		description: "위젯에 캐릭터를 표시합니다.",
		icon: "User",
		defaultEnabled: false,
		version: "1.0",
		minAppVersion: "1.0",
	},

	{
		id: "widget.music",
		name: "음악 플레이어",
		description: "위젯에서 음악을 제어합니다.",
		icon: "Music",
		defaultEnabled: false,
		version: "1.0",
		minAppVersion: "1.0",
	},

	{
		id: "widget.clock",
		name: "시계",
		description: "위젯에 현재 시간을 표시합니다.",
		icon: "Clock",
		defaultEnabled: false,
		version: "1.0",
		minAppVersion: "1.0",
	},

	{
		id: "widget.memo",
		name: "메모",
		description: "간단한 메모 기능을 제공합니다.",
		icon: "StickyNote",
		defaultEnabled: false,
		version: "1.0",
		minAppVersion: "1.0",
	},
];

