import { contextMenuRegistry } from "$/lib/contextmenu/registry";
import { contextActionRegistry } from "$/lib/contextmenu/actions";
import Simeji from "./Simeji.svelte";
import type { FeatureRuntime } from "$/lib/features/types";
import { widgetFeatures } from "$/lib/features/widgetFeatures.svelte";
import type { ContextMenuContext } from "$/lib/contextmenu/types";
import { Character } from "./character/Character";

export const WIDGET_MASCOT_FEATURE_ID = "widget.mascot";

class RegisterMascotFeature implements FeatureRuntime {
	id = WIDGET_MASCOT_FEATURE_ID;
	enabled = $state(false);
	defaultEnabled = false;
	component = Simeji;
	private unsubscriber?: () => void;

	async init() {
		if (!this.enabled) return
		this.enabled = true;
		this.unsubscriber = registerMascotMenuFeature();
	}

	shown() {
		this.enabled = true;
	}

	disable() {
		this.enabled = false;
	}
	
	destroy() {
		this.enabled = false;
		this.unsubscriber?.();
		this.unsubscriber = undefined;

		// 제거
		widgetFeatures.unregister(this.id);
	}
}

export const registerWidgetMascot = new RegisterMascotFeature();

export function registerMascotMenuFeature() {
	const unregisterMenu = contextMenuRegistry.register({
		featureId: WIDGET_MASCOT_FEATURE_ID,
		priority: 10,
		interactive: true,

		items: [
			{
				id: "mascot.chat",
				label: "대화하기",
				icon: "MessageCircle",
				action: "mascot.chat",
			},

			{
				id: "mascot.action",
				label: "행동",
				icon: "Sparkle",
				children: [
					{
						id: "mascot.action.free",
						label: "자유행동",
						action: "mascot.action.free",
					},
					{
						id: "mascot.action.sitting",
						label: "앉아있기",
						action: "mascot.action.sitting",
					},
					{
						id: "mascot.action.sleeping",
						label: "잠자기",
						action: "mascot.action.sleeping",
					},
				]
			},

			{
				id: "mascot.change",
				label: "캐릭터 변경",
				icon: "User",
				action: "mascot.change",
			},

			{
				id: "mascot.separator",
				label: "",
				separator: true,
			},

			{
				id: "mascot.hide",
				label: "숨기기",
				icon: "EyeOff",
				action: "mascot.hide",
			},

			{
				id: "mascot.quit",
				label: "종료",
				icon: "X",
				action: "mascot.quit",
			},
		],
	});

	const unregisterChat = contextActionRegistry.register("mascot.chat", (context?: ContextMenuContext) => {
		console.log("Open chat");
		const character = context?.data?.char;
		if (character instanceof Character) {
			character.setManualBehavior('working');
		}
	});

	const unregisterChange = contextActionRegistry.register("mascot.change", () => {
		console.log("Change mascot");
	});
	const unregisterMascotFree = contextActionRegistry.register("mascot.action.free", (context?: ContextMenuContext) => {
		const character = context?.data?.char;
		if (character instanceof Character) {
			character.resumeAutomaticBehavior();
		}
	});
	const unregisterMascotSitting = contextActionRegistry.register("mascot.action.sitting", (context?: ContextMenuContext) => {
		const character = context?.data?.char;
		if (character instanceof Character) {
			character.setManualBehavior('sitting');
		}
	});
	const unregisterMascotSleep = contextActionRegistry.register("mascot.action.sleep", (context?: ContextMenuContext) => {
		const character = context?.data?.char;
		if (character instanceof Character) {
			character.setManualBehavior('sleep');
		}
	});

	const unregisterHide = contextActionRegistry.register("mascot.hide", () => {
		registerWidgetMascot.disable();
	});

	const unregisterQuit = contextActionRegistry.register("mascot.quit", () => {
		registerWidgetMascot.destroy();
	});

	return () => {
		unregisterMenu();
		unregisterChat();
		unregisterMascotFree();
		unregisterMascotSitting();
		unregisterMascotSleep();
		unregisterChange();
		unregisterHide();
		unregisterQuit();
	};
}
