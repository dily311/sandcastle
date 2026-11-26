import { get } from "svelte/store";
import { _ } from "svelte-i18n";
import { getAppLocale } from "$lib/utils/i18n";
import { LineKind, type GameDefinition, type GameEvent, type Location, type RenderState, type StageLine } from "$features/game/types";

const t = (k: string, values?: Record<string, string | number>) => get(_)(k, values ? { values } : undefined);


/** 대사창에 보여줄 다음 할 일 하나 (이동·대화·살펴보기) */
export type PlayAction = {
	id: string;
	kind: "talk" | "look" | "move" | "wait";
	label: string;
	hint?: string;
	disabled?: boolean;
	run: () => void;
};

export function formatGameTime(seconds = 0): string {
	const totalMinutes = Math.floor(seconds / 60);
	const hours = Math.floor(totalMinutes / 60) % 24;
	const minutes = totalMinutes % 60;
	return `${String(hours).padStart(2, "0")}:${String(minutes).padStart(2, "0")}`;
}

export function isNarration(line: StageLine | null): boolean {
	return !line || line.kind === 0 || String(line.kind).toLowerCase() === "narration";
}

export function normalizeCueType(type: string | undefined): string {
	return (type ?? "").replace(/([a-z])([A-Z])/g, "$1_$2").toLowerCase();
}

export function getStageLines(events: GameEvent[]): StageLine[] {
	const sceneScript = [...events].reverse().find((event) => event.type === "sceneScript");
	return sceneScript?.type === "sceneScript"
		? sceneScript.lines
		: events.flatMap((event) => (event.type === "stageLines" ? event.lines : []));
}

/** 글 없이 연출만 있는 줄 (배경 전환, 캐릭터 내리기 등) */
export function isDirectionOnly(line: StageLine): boolean {
	return !line.text?.trim();
}

/**
 * 연출만 있는 줄을 대사 줄에 합친다. 화면에 "…"로 한 줄씩 보이지 않게 한다.
 * 연출은 다음 대사 줄을 보여주기 직전에 실행하고(제작자 대본 규칙과 같음), 뒤에 대사가 없으면 마지막 대사 줄에 붙인다.
 * 대사 줄이 하나도 없으면 빈 목록 (연출은 이미 무대 상태(stage)에 반영돼 있다).
 */
export function foldDirectionLines(lines: StageLine[]): StageLine[] {
	const out: StageLine[] = [];
	let pending: StageLine["cues"] = [];
	for (const line of lines) {
		if (isDirectionOnly(line)) {
			pending = [...pending, ...(line.cues ?? [])];
			continue;
		}
		out.push(pending.length ? { ...line, cues: [...pending, ...(line.cues ?? [])] } : line);
		pending = [];
	}
	if (pending.length && out.length) {
		const last = out[out.length - 1];
		out[out.length - 1] = { ...last, cues: [...(last.cues ?? []), ...pending] };
	}
	return out;
}

/** 받침이 있으면 "을", 없으면 "를" (한글이 아니면 "을(를)") */
export function objectParticle(word: string): string {
	if (!getAppLocale().startsWith("ko")) return "";
	const last = word.trim().slice(-1);
	const code = last.charCodeAt(0) - 0xac00;
	if (code < 0 || code > 11171) return "을(를)"; // i18n-ignore: 한국어 조사 계산
	return code % 28 === 0 ? "를" : "을"; // i18n-ignore: 한국어 조사 계산
}

/** 방향 조사: 받침이 없거나 ㄹ 받침이면 "로", 그 밖의 받침은 "으로" */
export function directionParticle(word: string): string {
	if (!getAppLocale().startsWith("ko")) return "";
	const last = word.trim().slice(-1);
	const code = last.charCodeAt(0) - 0xac00;
	if (code < 0 || code > 11171) return "(으)로"; // i18n-ignore: 한국어 조사 계산
	const jong = code % 28;
	return jong === 0 || jong === 8 ? "로" : "으로"; // i18n-ignore: 한국어 조사 계산
}

/** 아이템을 얻었다는 대사 줄 (대사창에 이어서 보여준다) */
export function itemGainLines(events: GameEvent[], items: { id: string; name: string }[] = []): StageLine[] {
	return getItemAddedEvents(events)
		.filter((e) => e.quantity > 0)
		.map((e) => {
			const name = items.find((item) => item.id === e.itemId)?.name ?? t("gamePlayUtils.itemFallback");
			const text =
				e.quantity > 1
					? t("gamePlayUtils.itemGainedCount", { name, count: e.quantity })
					: t("gamePlayUtils.itemGainedOne", { name, particle: objectParticle(name) });
			return { kind: LineKind.Narration, speaker: null, text, cues: [] };
		});
}

export function getItemAddedEvents(events: GameEvent[]) {
	return events.flatMap((event) => (event.type === "itemAdded" ? [event] : []));
}

export function createLocationArrivalLine(location: Location | undefined): StageLine | null {
	const description = location?.description.trim();
	if (!location || !description) return null;
	return { kind: LineKind.Narration, speaker: null, text: description, cues: [] };
}

export type ConnectionRequirement = { isMet: boolean; label: string };

function conditionValue(condition: Record<string, unknown>, snake: string, camel: string): unknown {
	return condition[snake] ?? condition[camel];
}

function formatDuration(seconds: number): string {
	if (seconds >= 3600) return t("gamePlayUtils.durationHours", { h: Math.ceil(seconds / 3600) });
	return t("gamePlayUtils.durationMinutes", { m: Math.ceil(seconds / 60) });
}

function formatMinuteOfDay(minute: number): string {
	return `${String(Math.floor(minute / 60) % 24).padStart(2, "0")}:${String(minute % 60).padStart(2, "0")}`;
}

export function getConnectionRequirement(
	rawCondition: string | null | undefined,
	state: RenderState | undefined,
	view: GameDefinition | undefined,
): ConnectionRequirement | null {
	if (!rawCondition?.trim()) return null;
	let condition: Record<string, unknown>;
	try {
		condition = JSON.parse(rawCondition) as Record<string, unknown>;
	} catch {
		return { isMet: false, label: t("gamePlayUtils.condNeedCheck") };
	}

	const type = String(condition.type ?? "")
		.replace(/([a-z])([A-Z])/g, "$1_$2")
		.toLowerCase();
	const key = String(condition.key ?? "");
	const value = condition.value;
	const inventory = (state?.inventory ?? {}) as Record<string, number>;
	const flags = (state?.flags ?? {}) as Record<string, unknown>;
	const relationships = state?.relationships ?? {};
	const itemName = (id: unknown) => view?.items.find((item) => item.id === String(id))?.name ?? t("gamePlayUtils.itemFallback");
	const locationName = (id: unknown) =>
		view?.locations.find((location) => location.id === String(id))?.name ?? t("gamePlayUtils.locationFallback");
	const zoneName = (id: unknown) => view?.zones.find((zone) => zone.id === String(id))?.name ?? t("gamePlayUtils.zoneFallback");
	const timePeriodName = (id: unknown) =>
		view?.timePeriods.find((period) => period.id === String(id))?.name ?? t("gamePlayUtils.timePeriodFallback");
	const weatherName = (id: unknown) =>
		view?.weatherTypes.find((weather) => weather.id === String(id))?.name ?? t("gamePlayUtils.weatherFallback");
	let isMet = false;
	let label = t("gamePlayUtils.condMet");

	switch (type) {
		case "flag_gte": {
			const required = Number(value);
			isMet = Number(flags[key] ?? 0) >= required;
			label = t("gamePlayUtils.condFlagGte", { key, required });
			break;
		}
		case "flag_eq":
			isMet = JSON.stringify(flags[key]) === JSON.stringify(value);
			label = `${key} = ${String(value)}`;
			break;
		case "min_time": {
			const seconds = Number(condition.seconds ?? 0);
			isMet = (state?.gameTime ?? 0) >= seconds;
			label = t("gamePlayUtils.condMinTime", { duration: formatDuration(seconds) });
			break;
		}
		case "in_location": {
			const id = conditionValue(condition, "location_id", "locationId");
			isMet = state?.locationId === id;
			label = t("gamePlayUtils.condInLocation", { place: locationName(id) });
			break;
		}
		case "in_zone": {
			const id = conditionValue(condition, "zone_id", "zoneId");
			isMet = state?.zoneId === id;
			label = t("gamePlayUtils.condInZone", { place: zoneName(id) });
			break;
		}
		case "has_item": {
			const id = conditionValue(condition, "item_id", "itemId");
			const min = Number(condition.min ?? 1);
			isMet = Number(inventory[String(id)] ?? 0) >= min;
			label = t("gamePlayUtils.condHasItem", { name: itemName(id), min });
			break;
		}
		case "time_period": {
			const id = conditionValue(condition, "period_id", "periodId");
			isMet = state?.timePeriodId === id;
			label = t("gamePlayUtils.condTimePeriod", { period: timePeriodName(id) });
			break;
		}
		case "weather": {
			const id = condition.weather_id ?? condition.weatherId;
			isMet = state?.weatherId === id;
			label = t("gamePlayUtils.condWeather", { weather: weatherName(id) });
			break;
		}
		case "relation_at_least":
		case "relation_at_most": {
			const from = String(condition.from ?? "player");
			const to = String(condition.to ?? "player");
			const axis = String(condition.axis ?? "affection");
			const relation = relationships[`${from}>${to}`];
			const current = Number(
				axis === "affection"
					? relation?.affection
					: axis === "trust"
						? relation?.trust
						: axis === "tension"
							? relation?.tension
							: 0,
			);
			const required = Number(value);
			isMet = type === "relation_at_least" ? current >= required : current <= required;
			const axisName =
				axis === "affection"
					? t("gamePlayUtils.axisAffection")
					: axis === "trust"
						? t("gamePlayUtils.axisTrust")
						: axis === "tension"
							? t("gamePlayUtils.axisTension")
							: axis;
			const fromName =
				from === "player" ? t("gameUi.meFallback") : (view?.characters.find((character) => character.id === from)?.name ?? from);
			const toName =
				to === "player" ? t("gameUi.meFallback") : (view?.characters.find((character) => character.id === to)?.name ?? to);
			label = t("gamePlayUtils.condRelation", {
				from: fromName,
				to: toName,
				axis: axisName,
				op: type === "relation_at_least" ? "≥" : "≤",
				required,
			});
			break;
		}
		case "at_minute_of_day": {
			const minute = Number(condition.minute ?? 0);
			label = t("gamePlayUtils.condAtTime", { time: formatMinuteOfDay(minute) });
			break;
		}
		case "never":
		default:
			label = t("gamePlayUtils.condSpecial");
	}
	return { isMet, label };
}
