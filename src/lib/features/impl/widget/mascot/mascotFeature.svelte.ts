import { contextMenuRegistry } from "$/lib/contextmenu/registry";
import { contextActionRegistry } from "$/lib/contextmenu/actions";
import { tr } from "$lib/utils/i18n";
import Simeji from "./Simeji.svelte";
import type { FeatureRuntime } from "$/lib/features/types";
import { widgetFeatureConfig, widgetFeatures } from "$/lib/features/widgetFeatures.svelte";
import type { ContextMenuContext } from "$/lib/contextmenu/types";
import { Character } from "./character/Character";
import type { WidgetGame } from "./pixi/WidgetGame";

export const WIDGET_MASCOT_FEATURE_ID = "widget.mascot";

/** Simeji.svelte가 구독하는 대화창/선물창 열기 요청 상태 */
class MascotUiBridge {
	activePrompt = $state<{ mascotId: string; mode: "chat" | "gift" } | null>(null);

	openChat(mascotId: string) {
		this.activePrompt = { mascotId, mode: "chat" };
	}
	openGift(mascotId: string) {
		this.activePrompt = { mascotId, mode: "gift" };
	}
	close() {
		this.activePrompt = null;
	}
}

export const mascotUi = new MascotUiBridge();

class RegisterMascotFeature implements FeatureRuntime {
	id = WIDGET_MASCOT_FEATURE_ID;
	enabled = $state(false);
	defaultEnabled = false;
	component = Simeji;
	private unsubscriber?: () => void;

	async init() {
		if (!this.enabled) return;
		this.enabled = true;
		this.unsubscriber?.();
		this.unsubscriber = registerMascotMenuFeature();
	}

	shown() {
		this.enabled = true;
	}

	disable() {
		this.enabled = false;
		mascotUi.close();
	}

	destroy() {
		this.enabled = false;
		mascotUi.close();
		this.unsubscriber?.();
		this.unsubscriber = undefined;
		widgetFeatures.unregister(this.id);
	}
}

export const registerWidgetMascot = new RegisterMascotFeature();

function getCtx(context?: ContextMenuContext): { char?: Character; game?: WidgetGame; mascotCount: number } {
	const data = context?.data as { char?: Character; game?: WidgetGame; mascotCount?: number } | undefined;
	return {
		char: data?.char instanceof Character ? data.char : undefined,
		game: data?.game,
		mascotCount: data?.mascotCount ?? 1,
	};
}

export function registerMascotMenuFeature() {
	const unregisterMenu = contextMenuRegistry.register({
		featureId: WIDGET_MASCOT_FEATURE_ID,
		priority: 10,
		interactive: true,
		items: [
			{
				id: "mascot.chat",
				label: "widgetExtra.mascotMenu.chat",
				icon: "MessageCircle",
				action: "mascot.chat",
			},
			{
				id: "mascot.pet",
				label: "widgetExtra.mascotMenu.pet",
				icon: "Heart",
				action: "mascot.pet",
			},
			{
				id: "mascot.gift",
				label: "widgetExtra.mascotMenu.gift",
				icon: "Gift",
				children: [
					{ id: "mascot.gift.cake", label: "widgetExtra.mascotMenu.giftCake", action: "mascot.gift.cake" },
					{ id: "mascot.gift.coffee", label: "widgetExtra.mascotMenu.giftCoffee", action: "mascot.gift.coffee" },
					{ id: "mascot.gift.flower", label: "widgetExtra.mascotMenu.giftFlower", action: "mascot.gift.flower" },
					{ id: "mascot.gift.candy", label: "widgetExtra.mascotMenu.giftCandy", action: "mascot.gift.candy" },
					{ id: "mascot.gift.custom", label: "widgetExtra.mascotMenu.giftCustom", action: "mascot.gift.custom" },
				],
			},
			{
				id: "mascot.peer",
				label: "widgetExtra.mascotMenu.peer",
				icon: "MessagesSquare",
				action: "mascot.peer",
				visible: (ctx) => getCtx(ctx).mascotCount >= 2,
			},
			{
				id: "mascot.action",
				label: "widgetExtra.mascotMenu.action",
				icon: "Sparkle",
				children: [
					{ id: "mascot.action.free", label: "widgetExtra.mascotMenu.actionFree", action: "mascot.action.free" },
					{ id: "mascot.action.sitting", label: "widgetExtra.mascotMenu.actionSitting", action: "mascot.action.sitting" },
					{ id: "mascot.action.sleep", label: "widgetExtra.mascotMenu.actionSleep", action: "mascot.action.sleep" },
					{ id: "mascot.action.working", label: "widgetExtra.mascotMenu.actionWorking", action: "mascot.action.working" },
				],
			},
			{
				id: "mascot.separator",
				label: "",
				separator: true,
			},
			{
				id: "mascot.hide",
				label: "widgetExtra.mascotMenu.hide",
				icon: "EyeOff",
				action: "mascot.hide",
			},
			{
				id: "mascot.quit",
				label: "widgetExtra.mascotMenu.quit",
				icon: "X",
				action: "mascot.quit",
			},
		],
	});

	const unsubscribers = [
		unregisterMenu,
		contextActionRegistry.register("mascot.chat", (context) => {
			const { char } = getCtx(context);
			if (char) mascotUi.openChat(char.id);
		}),
		contextActionRegistry.register("mascot.pet", (context) => {
			const { char, game } = getCtx(context);
			if (char && game) game.petMascot(char.id);
		}),
		contextActionRegistry.register("mascot.gift.cake", (context) => {
			const { char, game } = getCtx(context);
			if (char && game) void game.giveGiftToMascot(char.id, tr("mascotLib.giftCake"));
		}),
		contextActionRegistry.register("mascot.gift.coffee", (context) => {
			const { char, game } = getCtx(context);
			if (char && game) void game.giveGiftToMascot(char.id, tr("mascotLib.giftCoffee"));
		}),
		contextActionRegistry.register("mascot.gift.flower", (context) => {
			const { char, game } = getCtx(context);
			if (char && game) void game.giveGiftToMascot(char.id, tr("mascotLib.giftFlower"));
		}),
		contextActionRegistry.register("mascot.gift.candy", (context) => {
			const { char, game } = getCtx(context);
			if (char && game) void game.giveGiftToMascot(char.id, tr("mascotLib.giftCandy"));
		}),
		contextActionRegistry.register("mascot.gift.custom", (context) => {
			const { char } = getCtx(context);
			if (char) mascotUi.openGift(char.id);
		}),
		contextActionRegistry.register("mascot.peer", (context) => {
			const { char, game } = getCtx(context);
			if (char && game) void game.triggerPeerChat(char.id);
		}),
		contextActionRegistry.register("mascot.action.free", (context) => {
			getCtx(context).char?.resumeAutomaticBehavior();
		}),
		contextActionRegistry.register("mascot.action.sitting", (context) => {
			getCtx(context).char?.setManualBehavior("sitting");
		}),
		contextActionRegistry.register("mascot.action.sleep", (context) => {
			getCtx(context).char?.setManualBehavior("sleep");
		}),
		contextActionRegistry.register("mascot.action.working", (context) => {
			getCtx(context).char?.setManualBehavior("working");
		}),
		contextActionRegistry.register("mascot.hide", (context) => {
			const { char, game } = getCtx(context);
			if (char && game) void game.hideMascot(char.id);
		}),
		contextActionRegistry.register("mascot.quit", () => {
			void widgetFeatureConfig.updateConfig(WIDGET_MASCOT_FEATURE_ID, false);
			registerWidgetMascot.disable();
		}),
	];

	return () => {
		for (const u of unsubscribers) u();
	};
}
