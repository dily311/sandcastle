import type { Feature } from "$/lib/features/types";

export const WIDGET_FEATURES: Feature[] = [
	{
		id: "widget.mascot",
		name: "시메지 (마스코트)",
		description: "바탕화면에 시메지 캐릭터를 띄우고 상호작용합니다.",
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

	// {
	// 	id: "widget.clock",
	// 	name: "시계",
	// 	description: "위젯에 현재 시간을 표시합니다.",
	// 	icon: "Clock",
	// 	defaultEnabled: false,
	// 	version: "1.0",
	// 	minAppVersion: "1.0",
	// },

	// {
	// 	id: "widget.memo",
	// 	name: "메모",
	// 	description: "간단한 메모 기능을 제공합니다.",
	// 	icon: "StickyNote",
	// 	defaultEnabled: false,
	// 	version: "1.0",
	// 	minAppVersion: "1.0",
	// },
];

