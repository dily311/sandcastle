import { describe, expect, it } from "vitest";
import {
	buildBehaviorsFromWeights,
	createMascotConfig,
	defaultDialogues,
	defaultWeights,
	formatDialogueTemplate,
	isLongAbsence,
	pickDialogueLine,
	SEVEN_DAYS_MS,
} from "./simejiType";

describe("mascot simeji helpers", () => {
	it("detects 7+ days absence correctly", () => {
		const now = 1_700_000_000_000;
		expect(isLongAbsence(null, now)).toBe(false);
		expect(isLongAbsence(now - SEVEN_DAYS_MS + 1000, now)).toBe(false);
		expect(isLongAbsence(now - SEVEN_DAYS_MS, now)).toBe(true);
		expect(isLongAbsence(now - SEVEN_DAYS_MS * 2, now)).toBe(true);
	});

	it("formats dialogue templates with user, char, notify, gift macros", () => {
		const out = formatDialogueTemplate("{{user}}님! {{char}}가 {{notify}} 소식과 {{gift}}을 전해요", {
			user: "민수",
			char: "아리아",
			notify: "새 답장",
			gift: "케이크",
		});
		expect(out).toBe("민수님! 아리아가 새 답장 소식과 케이크을 전해요");
	});

	it("picks a non-empty dialogue line and returns null when empty", () => {
		const d = defaultDialogues();
		const line = pickDialogueLine(d, "greeting", { user: "테스터", char: "엘리" });
		expect(line).toBeTruthy();
		expect(line).not.toContain("{{user}}");

		const empty = { ...d, gift: ["   ", ""] };
		expect(pickDialogueLine(empty, "gift", { user: "테스터" })).toBeNull();
	});

	it("builds weighted behaviors from mascot weights", () => {
		const weights = { ...defaultWeights(), walk: 60, sleep: 0 };
		const list = buildBehaviorsFromWeights(weights, 6);
		expect(list).toHaveLength(5);
		expect(list.find((b) => b.type === "walk")?.weight).toBe(60);
		expect(list.find((b) => b.type === "sleep")?.weight).toBe(0);
	});

	it("creates mascot config with sane defaults", () => {
		const c = createMascotConfig({ name: "세라" });
		expect(c.name).toBe("세라");
		expect(c.enabled).toBe(true);
		expect(c.scale).toBe(1);
		expect(c.dialogues.longAbsence.length).toBeGreaterThan(0);
	});
});
