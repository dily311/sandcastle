// 백엔드(Rust)에서 넘어오는 오류(key+args)·플러그인·작업/할당량 라벨·가져오기 경고를 현재 앱 언어(ko / en)에 맞게 변환한다 (ADR-032 ④, 5-C-2-9).
import { get } from "svelte/store";
import { _ } from "svelte-i18n";
import type { CorePlugin, ErrorCode } from "$ipc/bindings";
import { getAppLocale } from "./i18n";

const HAS_HANGUL = /[가-힣]/; // i18n-ignore: 한글 범위 정규식

function t(key: string, values?: Record<string, string | number>): string {
	const fn = get(_);
	return fn(key, values ? { values } : undefined);
}

function tryKey(key: string, values?: Record<string, string | number>): string | null {
	const out = t(key, values);
	return out === key ? null : out;
}

function localizeArgValue(val: string): string {
	if (!val || getAppLocale() === "ko") return val;
	const mapped = tryKey(`errors.argMap.${val}`);
	if (mapped) return mapped;
	return val;
}

function resolveStructuredKey(key: string, rawArgs?: Partial<Record<string, string>> | null): string | null {
	const args: Record<string, string> = {};
	for (const [k, v] of Object.entries(rawArgs ?? {})) {
		if (typeof v === "string") args[k] = localizeArgValue(v);
	}
	// 래퍼 메시지(플러그인 manifest.json: ..., 플러그인 묶음: ...)의 안쪽 subKey 먼저 번역
	if (args.subKey) {
		const inner = tryKey(args.subKey, args);
		if (inner) args.msg = inner;
	}
	for (const field of ["why", "reason"] as const) {
		const sk = args[`subKey_${field}`];
		if (sk) {
			const subArgs: Record<string, string> = {};
			const prefix = `sub_${field}_`;
			for (const [k, v] of Object.entries(args)) {
				if (k.startsWith(prefix)) subArgs[k.slice(prefix.length)] = v;
			}
			const inner = tryKey(sk, subArgs);
			if (inner) args[field] = inner;
		}
	}
	return tryKey(key, args);
}

/**
 * 백엔드에서 내려온 오류 응답(code + key + args + message + details)을 현재 UI 언어로 변환한다.
 * - 한국어(ko): 백엔드 원문(message)이 있으면 그대로 쓰고, 비어 있으면 key → ErrorCode 기본 문구 순
 * - 그 외 언어(en 등):
 *   1) 코어 플러그인 비활성화 오류(`details.plugin` 또는 `errors.pluginDisabled`)면 번역된 플러그인 이름으로 안내
 *   2) 백엔드가 채운 구조화 `key` + `args`가 있으면 정확한 인자 포함 문구로 치환
 *   3) `GAME_ACTION_BLOCKED`의 `details.reason`이 있으면 막힌 사유별 문구로 치환
 *   4) 메시지에 한글이 남아 있으면 `errors.<code>` 번역 문구로 치환, 순수 영문 공급자 오류는 그대로 유지
 */
export function localizeBackendError(err: {
	code?: ErrorCode | "UNKNOWN" | string;
	message?: string | null;
	key?: string | null;
	args?: Partial<Record<string, string>> | null;
	details?: unknown;
}): string {
	const raw = (err.message ?? "").trim();
	const code = err.code || "UNKNOWN";

	// 플러그인 비활성화 오류는 details.plugin 또는 args.plugin을 이용해 현재 언어로 조합
	const pluginId =
		(err.details as { plugin?: unknown } | null | undefined)?.plugin ??
		(err.key === "errors.pluginDisabled" ? err.args?.plugin : undefined);
	if (typeof pluginId === "string" && pluginId) {
		const pluginName = localizeCorePluginName(pluginId, pluginId);
		return t("errors.pluginDisabled", { name: pluginName });
	}

	if (getAppLocale() === "ko") {
		if (raw) return raw;
		if (err.key) {
			const structured = resolveStructuredKey(err.key, err.args);
			if (structured) return structured;
		}
		return tryKey(`errors.${code}`) ?? t("errors.UNKNOWN");
	}

	if (err.key) {
		const structured = resolveStructuredKey(err.key, err.args);
		if (structured) return structured;
	}

	if (code === "GAME_ACTION_BLOCKED") {
		const reason = (err.details as { reason?: unknown } | null | undefined)?.reason;
		if (typeof reason === "string" && reason) {
			const byReason = tryKey(`errors.gameBlocked.${reason}`);
			if (byReason) return byReason;
		}
	}

	if (!raw || HAS_HANGUL.test(raw)) {
		return tryKey(`errors.${code}`) ?? t("errors.UNKNOWN");
	}

	return raw;
}

const pluginKey = (id: string) => id.replace(/\./g, "_");

export function localizeCorePluginName(id: string, fallback: string): string {
	return tryKey(`corePlugins.items.${pluginKey(id)}.name`) ?? fallback;
}

export function localizeCorePluginDesc(id: string, fallback: string): string {
	return tryKey(`corePlugins.items.${pluginKey(id)}.desc`) ?? fallback;
}

export function localizeCorePlugin(p: CorePlugin): { name: string; description: string } {
	return {
		name: localizeCorePluginName(p.id, p.name),
		description: localizeCorePluginDesc(p.id, p.description),
	};
}

export function localizeTaskFeature(feature: string | null | undefined): string {
	if (!feature) return t("tasks.feature.default");
	if (feature.startsWith("plugin:")) return t("tasks.feature.plugin");
	return tryKey(`tasks.feature.${feature}`) ?? feature;
}

export function localizeTaskStatus(status: string): string {
	return tryKey(`tasks.status.${status}`) ?? status;
}

// i18n-ignore: 백엔드 CreateTaskRequest.label 원문 매핑표
const TASK_LABEL_MAP: Record<string, string> = {
	"응답 생성": "replyGen", // i18n-ignore
	"이어 쓰기": "continueReply", // i18n-ignore
	"지시와 함께 다시 생성": "regenWithNote", // i18n-ignore
	"다른 응답": "newSwipe", // i18n-ignore
	"다시 생성": "regenerate", // i18n-ignore
	"엿보기": "peek", // i18n-ignore
	"API 연결 확인": "apiCheck", // i18n-ignore
	"각본가": "storyPlanner", // i18n-ignore
	"대필": "ghostwrite", // i18n-ignore
	"메모리 검색": "memorySearch", // i18n-ignore
	"메모리 색인 · 이 대화": "memoryIndexConv", // i18n-ignore
	"메모리 색인 · 전체": "memoryIndexAll", // i18n-ignore
	"메모리 회상": "memoryRecall", // i18n-ignore
	"폰 개인 페이지 생성": "phonePageGen", // i18n-ignore
	"음성 읽기": "ttsRead", // i18n-ignore
	"메시지 그림": "messageImage", // i18n-ignore
	"자동 그림": "autoImage", // i18n-ignore
	"게임 장면 그림": "gameSceneImage", // i18n-ignore
	"폰 카메라": "phoneCamera", // i18n-ignore
	"그림 만들기": "createImage", // i18n-ignore
	"시메지 대사": "mascotLine", // i18n-ignore
	"시메지끼리 대화": "mascotPeerChat", // i18n-ignore
	"게임 대사": "gameDialogue", // i18n-ignore
	"동기화 파일 만들기": "syncExport", // i18n-ignore
	"동기화 파일 합치기": "syncMerge", // i18n-ignore
	"구글 드라이브 동기화": "syncDrive", // i18n-ignore
	"폴더 동기화": "syncFolder", // i18n-ignore
	"드라이브 데이터 지우기": "driveWipe", // i18n-ignore
};

/** 백엔드가 저장한 작업 `label`을 현재 언어로 변환한다 */
export function localizeTaskLabel(label: string | null | undefined, fallback = ""): string {
	const raw = (label ?? "").trim();
	if (!raw) return fallback;
	if (getAppLocale() === "ko") return raw;
	const exact = TASK_LABEL_MAP[raw];
	if (exact) return tryKey(`tasks.labels.${exact}`) ?? raw;
	const trMatch = /^번역 → (.+)$/.exec(raw); // i18n-ignore
	if (trMatch) return tryKey("tasks.labels.translateTo", { to: trMatch[1] }) ?? raw;
	const rwMatch = /^자동 재생성 \((\d+)\/(\d+)\)$/.exec(raw); // i18n-ignore
	if (rwMatch) return tryKey("tasks.labels.autoRewrite", { attempt: rwMatch[1], max: rwMatch[2] }) ?? raw;
	return raw;
}

const QUOTA_LABEL_MAP: Record<string, string> = {
	"구독": "subscription", // i18n-ignore
	"무제한 그림": "unlimitedImages", // i18n-ignore
	"기타": "other", // i18n-ignore
	"남은 크레딧": "remainingCredits", // i18n-ignore
	"사용": "used", // i18n-ignore
	"요금제": "plan", // i18n-ignore
	"초기화": "resets", // i18n-ignore
};

const QUOTA_VALUE_MAP: Record<string, string> = {
	"무료": "free", // i18n-ignore
	"구독 정보 없음": "noSubscription", // i18n-ignore
	"가능": "yes", // i18n-ignore
	"아님": "no", // i18n-ignore
};

/** 구독형 공급자 잔여(`QuotaLine`)의 라벨·값을 현재 언어로 변환한다 */
export function localizeQuotaLine(line: { label: string; value: string }): { label: string; value: string } {
	if (getAppLocale() === "ko") return line;
	const lk = QUOTA_LABEL_MAP[line.label];
	const vk = QUOTA_VALUE_MAP[line.value];
	return {
		label: lk ? (tryKey(`quota.labels.${lk}`) ?? line.label) : line.label,
		value: vk ? (tryKey(`quota.values.${vk}`) ?? line.value) : line.value,
	};
}

/** `test_message` 연결 확인 응답 문자열을 현재 언어로 변환한다 */
export function localizeApiTestReply(reply: string): string {
	const r = reply.trim();
	if (!r || getAppLocale() === "ko") return r;
	const voices = /^목소리 (\d+)개$/.exec(r); // i18n-ignore
	if (voices) return tryKey("quota.apiTestVoices", { n: voices[1] }) ?? r;
	const models = /^모델 (\d+)개 확인됨$/.exec(r); // i18n-ignore
	if (models) return tryKey("quota.apiTestModels", { n: models[1] }) ?? r;
	const nai = /^(무료|구독 정보 없음) · (Anlas .+)$/.exec(r); // i18n-ignore
	if (nai) {
		const tierKey = QUOTA_VALUE_MAP[nai[1]];
		const tier = tierKey ? (tryKey(`quota.values.${tierKey}`) ?? nai[1]) : nai[1];
		return `${tier} · ${nai[2]}`;
	}
	return r;
}

const IMPORT_WARNING_EXACT: Record<string, string> = {
	"post_history_instructions(기록 뒤 지시)는 지원하지 않아 가져오지 않음": "postHistoryUnsupported", // i18n-ignore
	"depth_prompt(캐릭터 노트)는 지원하지 않아 가져오지 않음": "depthPromptUnsupported", // i18n-ignore
	"작가 노트의 빈도·위치는 지원하지 않아 '대화 안 깊이'로 가져옴 (원본은 대화 정보에 보존)": "authorNoteDepthFallback", // i18n-ignore
	"성격(charPersonality)/시나리오(scenario) 블록은 비워 둠 — 캐릭터를 가져올 때 두 항목은 캐릭터 설명에 합쳐짐": "presetCharScenarioEmpty", // i18n-ignore
	"프롬프트 블록이 없어 기본 배치로 동작함": "presetNoBlocksFallback", // i18n-ignore
	"assistant prefill은 아직 지원하지 않아 원본에만 보존함": "presetPrefillPreserved", // i18n-ignore
};

const IMPORT_WARNING_PATTERNS: [RegExp, string, string[]][] = [
	[/^메시지 (\d+)개의 스와이프 정보가 맞지 않아 현재 표시된 내용만 가져옴$/, "brokenSwipes", ["n"]], // i18n-ignore
	[/^'(.+)': 자동 실행 설정은 가져오지 않음 — 누를 때만 실행됨$/, "qrItemAutoIgnored", ["label"]], // i18n-ignore
	[/^자동 실행 설정 (\d+)개는 가져오지 않음 — 누를 때만 실행됨$/, "qrCountAutoIgnored", ["n"]], // i18n-ignore
	[/^정규식 '(.+)': 찾을 패턴이 비어 있어 건너뜀$/, "regexEmptyPattern", ["name"]], // i18n-ignore
	[/^정규식 '(.+)': 슬래시 커맨드\/추론 등 지원하지 않는 적용 위치는 제외함$/, "regexUnsupportedPlacement", ["name"]], // i18n-ignore
	[/^정규식 '(.+)': 지원하는 적용 위치가 없어 건너뜀$/, "regexNoSupportedPlacement", ["name"]], // i18n-ignore
	[/^월드인포 '(.+)': 키워드가 없고 상시 항목도 아니라서 끈 상태로 가져옴$/, "worldInfoDisabledNoKey", ["name"]], // i18n-ignore
	[/^캐릭터를 찾지 못한 화자: (.+) \(이름만 표시됨\)$/, "unmatchedSpeakers", ["names"]], // i18n-ignore
	[/^아바타 이미지를 저장하지 못함: (.+)$/, "avatarSaveFailed", ["err"]], // i18n-ignore
	[/^정규식 '(.+)'을\(를\) 건너뜀: (.+)$/, "regexSkippedErr", ["name", "err"]], // i18n-ignore
	[/^'(.+)'의 변수 정의를 읽지 못해 뺌: (.+)$/, "packVarDefsSkipped", ["name", "why"]], // i18n-ignore
	[/^'(.+)'의 로어북 하나가 팩에 없어 연결하지 못함$/, "packMissingLorebook", ["name"]], // i18n-ignore
	[/^캐릭터가 팩에 없는 대화 '(.+)'를 건너뜀$/, "packOrphanConvSkipped", ["title"]], // i18n-ignore
	[/^대화 '(.+)'에 붙은 로어북 일부는 팩에 없어 빠짐$/, "packConvMissingLorebook", ["title"]], // i18n-ignore
	[/^chats\/([^:]+): 카드가 없어 이름만 있는 캐릭터 '(.+)'를 만들어 붙임$/, "stPlaceholderCharCreated", ["folder", "name"]], // i18n-ignore
	[/^chats\/([^:]+): 캐릭터를 만들지 못함 \((.+)\)$/, "stCreateCharFailed", ["folder", "err"]], // i18n-ignore
	[/^chats\/([^:]+): 분기 연결 못 함 \((.+)\)$/, "stBranchLinkFailed", ["path", "err"]], // i18n-ignore
	[/^chats\/([^:]+): 부모 채팅 '(.+)'이 없어 분기로 잇지 못함$/, "stBranchParentMissing", ["path", "parent"]], // i18n-ignore
	[/^(.+): 로어북이 아님$/, "stNotLorebook", ["path"]], // i18n-ignore
	[/^(.+): 로어북 연결 못 함 \((.+)\)$/, "stLorebookLinkFailed", ["path", "err"]], // i18n-ignore
	[/^(.+): 캐릭터 카드가 아님$/, "stNotCharCard", ["path"]], // i18n-ignore
	[/^(.+): 프리셋이 아님$/, "stNotPreset", ["path"]], // i18n-ignore
	[/^(.+): (\d+)개 건너뜀 \(Chat Completion 프리셋만 가져옴\)$/, "stOtherPresetsSkipped", ["folder", "n"]], // i18n-ignore
	[/^settings\.json 정규식: (.+)$/, "stSettingsRegexErr", ["err"]], // i18n-ignore
	[/^페르소나 (.+) 그림: (.+)$/, "stExportPersonaAvatarErr", ["name", "err"]], // i18n-ignore
	[/^페르소나 ([^:]+): (.+)$/, "stPersonaErr", ["name", "err"]], // i18n-ignore
	[/^캐릭터 ([^:]+): (.+)$/, "stExportCharErr", ["name", "err"]], // i18n-ignore
	[/^프리셋 ([^:]+): (.+)$/, "stExportPresetErr", ["name", "err"]], // i18n-ignore
	[/^단체 채팅 (\d+)개 건너뜀 \(지원하지 않음\)$/, "stGroupsSkipped", ["n"]], // i18n-ignore
	[/^(.+): (\d+)MB가 넘어 건너뜀$/, "stOversizedSkipped", ["name", "mb"]], // i18n-ignore
	[/^(.+): 압축 비율이 비정상이라 건너뜀$/, "stZipRatioSkipped", ["name"]], // i18n-ignore
];

/** SillyTavern·캐릭터 팩 가져오기/내보내기 경고 원문을 현재 언어로 변환한다 */
export function localizeImportWarning(warning: string): string {
	const w = warning.trim();
	if (!w || getAppLocale() === "ko") return w;

	const exact = IMPORT_WARNING_EXACT[w];
	if (exact) return tryKey(`importWarnings.${exact}`) ?? w;

	if (w.startsWith("정규식: ")) { // i18n-ignore
		const inner = localizeImportWarning(w.slice("정규식: ".length)); // i18n-ignore
		return tryKey("importWarnings.stRegexPrefix", { msg: inner }) ?? `Regex: ${inner}`;
	}

	for (const [re, id, names] of IMPORT_WARNING_PATTERNS) {
		const m = re.exec(w);
		if (m) {
			const values: Record<string, string> = {};
			names.forEach((n, idx) => {
				const v = m[idx + 1] ?? "";
				values[n] = n === "err" ? localizeBackendError({ code: "INVALID_INPUT", message: v }) : v;
			});
			return tryKey(`importWarnings.${id}`, values) ?? w;
		}
	}

	return w;
}
