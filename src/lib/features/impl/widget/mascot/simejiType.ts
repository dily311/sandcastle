import type {
	MascotBehaviorWeights_Serialize,
	MascotCharacterConfig_Serialize,
	MascotDialogues_Serialize,
	MascotImages_Serialize,
	WidgetMascotSetting_Serialize,
} from "$ipc/bindings";

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
	{ key: "idle", label: "기본 (대기)", hint: "평소 가만히 있을 때" },
	{ key: "walk", label: "걷기", hint: "화면을 돌아다닐 때" },
	{ key: "sitting", label: "앉기", hint: "앉아서 쉴 때" },
	{ key: "sleep", label: "잠자기", hint: "졸거나 잠들었을 때" },
	{ key: "drag", label: "잡아 끌기", hint: "마우스로 들어올렸을 때" },
	{ key: "happy", label: "행복", hint: "쓰다듬기·선물·기분 좋을 때" },
	{ key: "annoyed", label: "짜증", hint: "자꾸 끌거나 귀찮게 했을 때" },
	{ key: "working", label: "대화 / 활동", hint: "대화하거나 알림을 전할 때" },
];

export const MASCOT_DIALOGUE_FIELDS: { key: MascotDialogueKey; label: string; hint: string }[] = [
	{ key: "greeting", label: "그리팅 (기본 인사)", hint: "위젯에 처음 등장했을 때 · {{user}}, {{char}} 사용 가능" },
	{ key: "longAbsence", label: "오랜만에 볼 경우 (7일 이상)", hint: "마지막 접속 후 7일 이상 지나서 만났을 때" },
	{ key: "gift", label: "선물을 준 경우", hint: "우클릭 메뉴로 선물을 주었을 때" },
	{ key: "notify", label: "알림이 온 경우", hint: "응답·작업 알림이 도착했을 때 · {{notify}}로 내용 표시" },
	{ key: "drag", label: "잡아 끈 경우", hint: "마우스로 시메지를 잡아 끌었을 때" },
	{ key: "annoyed", label: "짜증 난 경우", hint: "짧은 시간에 여러 번 잡아 끌거나 귀찮게 했을 때" },
	{ key: "happy", label: "행복해한 경우", hint: "쓰다듬거나 더블클릭으로 예뻐해 줬을 때" },
	{ key: "idleChatter", label: "평소 혼잣말", hint: "활동 중에 가끔 혼자 중얼거리는 대사" },
	{ key: "peerChat", label: "다른 시메지와 대화 (오프라인/기본)", hint: "AI 미사용 시 다른 시메지와 주고받는 대사" },
];

export function defaultDialogues(): MascotDialogues_Serialize {
	return {
		greeting: ["안녕, {{user}}!", "오늘도 좋은 하루 보내자!"],
		longAbsence: ["{{user}}! 정말 오랜만이야, 보고 싶었어!", "일주일 넘게 어디 갔었어? 기다렸잖아!"],
		gift: ["와, 고마워 {{user}}! 소중히 간직할게!", "선물이야? 정말 기뻐!"],
		notify: ["{{user}}, 알림이 왔어! {{notify}}", "띵동~ 새 소식이 도착했어!"],
		drag: ["으악, 잠깐만! 내려놔줘!", "어디로 데려가는 거야~?"],
		annoyed: ["그만 좀 괴롭혀! 어지럽단 말이야!", "흥, 자꾸 그러면 토라질 거야!"],
		happy: ["헤헤, 기분 좋아!", "역시 {{user}} 옆이 제일 좋아!"],
		idleChatter: ["심심한데 뭐 하고 놀까?", "오늘 날씨는 어떨까~"],
		peerChat: ["안녕! 오늘 기분은 어때?", "같이 화면 좀 산책할래?"],
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
		name: partial.name ?? "새 시메지",
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
		pairChatUseAi: true,
		pairChatDistancePx: 260,
	};
}

export function buildBehaviorsFromWeights(weights: MascotBehaviorWeights_Serialize, intervalSec: number): Behavior[] {
	const baseMs = Math.max(1500, intervalSec * 1000);
	return [
		{ id: "idle", name: "가만히 있기", type: "idle", weight: Math.max(0, weights.idle), minDuration: baseMs * 0.6, maxDuration: baseMs * 1.3 },
		{ id: "walk", name: "걸어다니기", type: "walk", weight: Math.max(0, weights.walk), minDuration: baseMs * 0.7, maxDuration: baseMs * 1.4 },
		{ id: "sitting", name: "앉아있기", type: "sitting", weight: Math.max(0, weights.sitting), minDuration: baseMs * 0.9, maxDuration: baseMs * 1.8 },
		{ id: "sleep", name: "잠자기", type: "sleep", weight: Math.max(0, weights.sleep), minDuration: baseMs * 1.2, maxDuration: baseMs * 2.2 },
		{ id: "working", name: "활동하기", type: "working", weight: Math.max(0, weights.working), minDuration: baseMs * 0.8, maxDuration: baseMs * 1.5 },
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
		.replace(/\{\{user\}\}/gi, vars.user || "유저")
		.replace(/\{\{char\}\}/gi, vars.char || "시메지")
		.replace(/\{\{notify\}\}/gi, vars.notify || "")
		.replace(/\{\{gift\}\}/gi, vars.gift || "선물")
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
