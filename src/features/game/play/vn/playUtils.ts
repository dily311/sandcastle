import { LineKind, type GameDefinition, type GameEvent, type Location, type RenderState, type StageLine } from "$features/game/types";

export type ItemGainNotice = { id: number; name: string; quantity: number };

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
	if (seconds >= 3600) return `${Math.ceil(seconds / 3600)}시간`;
	return `${Math.ceil(seconds / 60)}분`;
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
		return { isMet: false, label: "조건 확인 필요" };
	}

	const type = String(condition.type ?? "")
		.replace(/([a-z])([A-Z])/g, "$1_$2")
		.toLowerCase();
	const key = String(condition.key ?? "");
	const value = condition.value;
	const inventory = (state?.inventory ?? {}) as Record<string, number>;
	const flags = (state?.flags ?? {}) as Record<string, unknown>;
	const relationships = state?.relationships ?? {};
	const itemName = (id: unknown) => view?.items.find((item) => item.id === String(id))?.name ?? "아이템";
	const locationName = (id: unknown) =>
		view?.locations.find((location) => location.id === String(id))?.name ?? "장소";
	const zoneName = (id: unknown) => view?.zones.find((zone) => zone.id === String(id))?.name ?? "지역";
	const timePeriodName = (id: unknown) =>
		view?.timePeriods.find((period) => period.id === String(id))?.name ?? "시간대";
	const weatherName = (id: unknown) =>
		view?.weatherTypes.find((weather) => weather.id === String(id))?.name ?? "날씨";
	let isMet = false;
	let label = "조건 충족";

	switch (type) {
		case "flag_gte": {
			const required = Number(value);
			isMet = Number(flags[key] ?? 0) >= required;
			label = `${key} ${required} 이상`;
			break;
		}
		case "flag_eq":
			isMet = JSON.stringify(flags[key]) === JSON.stringify(value);
			label = `${key} = ${String(value)}`;
			break;
		case "min_time": {
			const seconds = Number(condition.seconds ?? 0);
			isMet = (state?.gameTime ?? 0) >= seconds;
			label = `누적 시간 ${formatDuration(seconds)} 이상`;
			break;
		}
		case "in_location": {
			const id = conditionValue(condition, "location_id", "locationId");
			isMet = state?.locationId === id;
			label = `${locationName(id)}에 있음`;
			break;
		}
		case "in_zone": {
			const id = conditionValue(condition, "zone_id", "zoneId");
			isMet = state?.zoneId === id;
			label = `${zoneName(id)}에 있음`;
			break;
		}
		case "has_item": {
			const id = conditionValue(condition, "item_id", "itemId");
			const min = Number(condition.min ?? 1);
			isMet = Number(inventory[String(id)] ?? 0) >= min;
			label = `${itemName(id)} ${min}개 보유`;
			break;
		}
		case "time_period": {
			const id = conditionValue(condition, "period_id", "periodId");
			isMet = state?.timePeriodId === id;
			label = `${timePeriodName(id)} 시간대`;
			break;
		}
		case "weather": {
			const id = condition.weather_id ?? condition.weatherId;
			isMet = state?.weatherId === id;
			label = `${weatherName(id)} 날씨`;
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
				axis === "affection" ? "호감도" : axis === "trust" ? "신뢰도" : axis === "tension" ? "긴장도" : axis;
			const fromName =
				from === "player" ? "나" : (view?.characters.find((character) => character.id === from)?.name ?? from);
			const toName =
				to === "player" ? "나" : (view?.characters.find((character) => character.id === to)?.name ?? to);
			label = `${fromName}의 ${toName}에 대한 ${axisName} ${type === "relation_at_least" ? "≥" : "≤"} ${required}`;
			break;
		}
		case "at_minute_of_day": {
			const minute = Number(condition.minute ?? 0);
			label = `${formatMinuteOfDay(minute)}에 도달`;
			break;
		}
		case "never":
		default:
			label = "특수 조건 충족";
	}
	return { isMet, label };
}
