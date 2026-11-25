import { contextMenuRegistry } from "$/lib/contextmenu/registry";
import { contextActionRegistry } from "$/lib/contextmenu/actions";
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
				label: "대화하기",
				rawLabel: true,
				icon: "MessageCircle",
				action: "mascot.chat",
			},
			{
				id: "mascot.pet",
				label: "쓰다듬기",
				rawLabel: true,
				icon: "Heart",
				action: "mascot.pet",
			},
			{
				id: "mascot.gift",
				label: "선물 주기",
				rawLabel: true,
				icon: "Gift",
				children: [
					{ id: "mascot.gift.cake", label: "🍰 케이크", rawLabel: true, action: "mascot.gift.cake" },
					{ id: "mascot.gift.coffee", label: "☕ 커피", rawLabel: true, action: "mascot.gift.coffee" },
					{ id: "mascot.gift.flower", label: "💐 꽃다발", rawLabel: true, action: "mascot.gift.flower" },
					{ id: "mascot.gift.candy", label: "🍬 별사탕", rawLabel: true, action: "mascot.gift.candy" },
					{ id: "mascot.gift.custom", label: "✏️ 직접 입력...", rawLabel: true, action: "mascot.gift.custom" },
				],
			},
			{
				id: "mascot.peer",
				label: "다른 시메지와 대화하기",
				rawLabel: true,
				icon: "MessagesSquare",
				action: "mascot.peer",
				visible: (ctx) => getCtx(ctx).mascotCount >= 2,
			},
			{
				id: "mascot.action",
				label: "행동",
				rawLabel: true,
				icon: "Sparkle",
				children: [
					{ id: "mascot.action.free", label: "자유행동", rawLabel: true, action: "mascot.action.free" },
					{ id: "mascot.action.sitting", label: "앉아있기", rawLabel: true, action: "mascot.action.sitting" },
					{ id: "mascot.action.sleep", label: "잠자기", rawLabel: true, action: "mascot.action.sleep" },
					{ id: "mascot.action.working", label: "활동하기", rawLabel: true, action: "mascot.action.working" },
				],
			},
			{
				id: "mascot.separator",
				label: "",
				separator: true,
			},
			{
				id: "mascot.hide",
				label: "이 시메지 숨기기",
				rawLabel: true,
				icon: "EyeOff",
				action: "mascot.hide",
			},
			{
				id: "mascot.quit",
				label: "시메지 모두 끄기",
				rawLabel: true,
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
			if (char && game) void game.giveGiftToMascot(char.id, "케이크");
		}),
		contextActionRegistry.register("mascot.gift.coffee", (context) => {
			const { char, game } = getCtx(context);
			if (char && game) void game.giveGiftToMascot(char.id, "따뜻한 커피");
		}),
		contextActionRegistry.register("mascot.gift.flower", (context) => {
			const { char, game } = getCtx(context);
			if (char && game) void game.giveGiftToMascot(char.id, "꽃다발");
		}),
		contextActionRegistry.register("mascot.gift.candy", (context) => {
			const { char, game } = getCtx(context);
			if (char && game) void game.giveGiftToMascot(char.id, "별사탕");
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
