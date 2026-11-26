import type {
	MascotBehaviorWeights_Serialize,
	MascotCharacterConfig_Serialize,
	MascotDialogues_Serialize,
	MascotImages_Serialize,
	WidgetMascotSetting_Serialize,
} from "$ipc/bindings";
import { tr } from "$lib/utils/i18n";

export type BehaviorMode = "automatic" | "manual";
export type BehaviorType = "idle" | "walk" | "sitting" | "sleep" | "working" | "drag" | "happy" | "annoyed";
export type CharacterAnimation = BehaviorType;
export type MascotImageSlot = keyof MascotImages_Serialize;
export type MascotDialogueKey = keyof MascotDialogues_Serialize;

export const SEVEN_DAYS_MS = 7 * 24 * 60 * 60 * 1000;

export interface Behavior {
	id: string;
	name: string;
	type: BehaviorType;
	weight: number;
	minDuration: number;
	maxDuration: number;
}

export interface MascotOverlayState {
	id: string;
	name: string;
	x: number;
	topY: number;
	bubbleText: string | null;
	bubbleSpeaker: string | null;
	bubbleMood: "normal" | "happy" | "annoyed" | "notify" | "peer";
}

export const MASCOT_IMAGE_SLOTS: { key: MascotImageSlot; label: string; hint: string }[] = [
	{ key: "idle", get label() { return tr("mascotLib.slots.idleLabel"); }, get hint() { return tr("mascotLib.slots.idleHint"); } },
	{ key: "walk", get label() { return tr("mascotLib.slots.walkLabel"); }, get hint() { return tr("mascotLib.slots.walkHint"); } },
	{ key: "sitting", get label() { return tr("mascotLib.slots.sittingLabel"); }, get hint() { return tr("mascotLib.slots.sittingHint"); } },
	{ key: "sleep", get label() { return tr("mascotLib.slots.sleepLabel"); }, get hint() { return tr("mascotLib.slots.sleepHint"); } },
	{ key: "drag", get label() { return tr("mascotLib.slots.dragLabel"); }, get hint() { return tr("mascotLib.slots.dragHint"); } },
	{ key: "happy", get label() { return tr("mascotLib.slots.happyLabel"); }, get hint() { return tr("mascotLib.slots.happyHint"); } },
	{ key: "annoyed", get label() { return tr("mascotLib.slots.annoyedLabel"); }, get hint() { return tr("mascotLib.slots.annoyedHint"); } },
	{ key: "working", get label() { return tr("mascotLib.slots.workingLabel"); }, get hint() { return tr("mascotLib.slots.workingHint"); } },
];

export const MASCOT_DIALOGUE_FIELDS: { key: MascotDialogueKey; label: string; hint: string }[] = [
	{ key: "greeting", get label() { return tr("mascotLib.dialogueFields.greetingLabel"); }, get hint() { return tr("mascotLib.dialogueFields.greetingHint"); } },
	{ key: "longAbsence", get label() { return tr("mascotLib.dialogueFields.longAbsenceLabel"); }, get hint() { return tr("mascotLib.dialogueFields.longAbsenceHint"); } },
	{ key: "gift", get label() { return tr("mascotLib.dialogueFields.giftLabel"); }, get hint() { return tr("mascotLib.dialogueFields.giftHint"); } },
	{ key: "notify", get label() { return tr("mascotLib.dialogueFields.notifyLabel"); }, get hint() { return tr("mascotLib.dialogueFields.notifyHint"); } },
	{ key: "drag", get label() { return tr("mascotLib.dialogueFields.dragLabel"); }, get hint() { return tr("mascotLib.dialogueFields.dragHint"); } },
	{ key: "annoyed", get label() { return tr("mascotLib.dialogueFields.annoyedLabel"); }, get hint() { return tr("mascotLib.dialogueFields.annoyedHint"); } },
	{ key: "happy", get label() { return tr("mascotLib.dialogueFields.happyLabel"); }, get hint() { return tr("mascotLib.dialogueFields.happyHint"); } },
	{ key: "idleChatter", get label() { return tr("mascotLib.dialogueFields.idleChatterLabel"); }, get hint() { return tr("mascotLib.dialogueFields.idleChatterHint"); } },
	{ key: "peerChat", get label() { return tr("mascotLib.dialogueFields.peerChatLabel"); }, get hint() { return tr("mascotLib.dialogueFields.peerChatHint"); } },
];

export function defaultDialogues(): MascotDialogues_Serialize {
	return {
		greeting: [tr("mascotLib.defaultLines.greeting1"), tr("mascotLib.defaultLines.greeting2")],
		longAbsence: [tr("mascotLib.defaultLines.longAbsence1"), tr("mascotLib.defaultLines.longAbsence2")],
		gift: [tr("mascotLib.defaultLines.gift1"), tr("mascotLib.defaultLines.gift2")],
		notify: [tr("mascotLib.defaultLines.notify1"), tr("mascotLib.defaultLines.notify2")],
		drag: [tr("mascotLib.defaultLines.drag1"), tr("mascotLib.defaultLines.drag2")],
		annoyed: [tr("mascotLib.defaultLines.annoyed1"), tr("mascotLib.defaultLines.annoyed2")],
		happy: [tr("mascotLib.defaultLines.happy1"), tr("mascotLib.defaultLines.happy2")],
		idleChatter: [tr("mascotLib.defaultLines.idleChatter1"), tr("mascotLib.defaultLines.idleChatter2")],
		peerChat: [tr("mascotLib.defaultLines.peerChat1"), tr("mascotLib.defaultLines.peerChat2")],
	};
}

export function defaultWeights(): MascotBehaviorWeights_Serialize {
	return { idle: 40, walk: 35, sitting: 10, sleep: 10, working: 5 };
}

export function defaultImages(): MascotImages_Serialize {
	return { idle: [], walk: [], sitting: [], sleep: [], drag: [], happy: [], annoyed: [], working: [] };
}

export function createMascotConfig(partial: Partial<MascotCharacterConfig_Serialize> = {}): MascotCharacterConfig_Serialize {
	return {
		id: partial.id ?? (typeof crypto !== "undefined" && "randomUUID" in crypto ? crypto.randomUUID() : `mascot-${Date.now()}-${Math.random().toString(36).slice(2, 7)}`),
		name: partial.name ?? tr("mascotLib.defaultNewMascotName"),
		enabled: partial.enabled ?? true,
		characterId: partial.characterId ?? null,
		personaId: partial.personaId ?? null,
		apiProfileId: partial.apiProfileId ?? null,
		model: partial.model ?? null,
		presetId: partial.presetId ?? null,
		useAiDialogue: partial.useAiDialogue ?? true,
		scale: partial.scale ?? 1.0,
		speed: partial.speed ?? 80,
		actionIntervalSec: partial.actionIntervalSec ?? 5,
		behaviorWeights: partial.behaviorWeights ? { ...defaultWeights(), ...partial.behaviorWeights } : defaultWeights(),
		images: partial.images ? { ...defaultImages(), ...partial.images } : defaultImages(),
		dialogues: partial.dialogues ? { ...defaultDialogues(), ...partial.dialogues } : defaultDialogues(),
		lastSeenAt: partial.lastSeenAt ?? null,
	};
}

export function defaultWidgetMascotSetting(): WidgetMascotSetting_Serialize {
	return {
		characters: [],
		pairChatEnabled: true,
		pairChatIntervalSec: 90,
		pairChatUseAi: false,
		pairChatDistancePx: 260,
	};
}

export function buildBehaviorsFromWeights(weights: MascotBehaviorWeights_Serialize, intervalSec: number): Behavior[] {
	const baseMs = Math.max(1500, intervalSec * 1000);
	return [
		{ id: "idle", name: tr("mascotLib.behaviors.idle"), type: "idle", weight: Math.max(0, weights.idle), minDuration: baseMs * 0.6, maxDuration: baseMs * 1.3 },
		{ id: "walk", name: tr("mascotLib.behaviors.walk"), type: "walk", weight: Math.max(0, weights.walk), minDuration: baseMs * 0.7, maxDuration: baseMs * 1.4 },
		{ id: "sitting", name: tr("mascotLib.behaviors.sitting"), type: "sitting", weight: Math.max(0, weights.sitting), minDuration: baseMs * 0.9, maxDuration: baseMs * 1.8 },
		{ id: "sleep", name: tr("mascotLib.behaviors.sleep"), type: "sleep", weight: Math.max(0, weights.sleep), minDuration: baseMs * 1.2, maxDuration: baseMs * 2.2 },
		{ id: "working", name: tr("mascotLib.behaviors.working"), type: "working", weight: Math.max(0, weights.working), minDuration: baseMs * 0.8, maxDuration: baseMs * 1.5 },
	];
}

export function isLongAbsence(lastSeenAt: number | null | undefined, now = Date.now()): boolean {
	if (!lastSeenAt || lastSeenAt <= 0) return false;
	return now - lastSeenAt >= SEVEN_DAYS_MS;
}

export function formatDialogueTemplate(
	template: string,
	vars: { user?: string; char?: string; notify?: string; gift?: string },
): string {
	return template
		.replace(/\{\{user\}\}/gi, vars.user || tr("mascotLib.defaultUserName"))
		.replace(/\{\{char\}\}/gi, vars.char || tr("mascotLib.defaultCharName"))
		.replace(/\{\{notify\}\}/gi, vars.notify || "")
		.replace(/\{\{gift\}\}/gi, vars.gift || tr("mascotLib.defaultGiftName"))
		.trim();
}

export function pickDialogueLine(
	dialogues: MascotDialogues_Serialize,
	key: MascotDialogueKey,
	vars: { user?: string; char?: string; notify?: string; gift?: string },
): string | null {
	const list = (dialogues[key] ?? []).map((s) => s.trim()).filter(Boolean);
	if (list.length === 0) return null;
	const chosen = list[Math.floor(Math.random() * list.length)];
	return formatDialogueTemplate(chosen, vars);
}
