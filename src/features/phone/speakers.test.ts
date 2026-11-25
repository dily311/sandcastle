import { describe, expect, it } from "vitest";
import { splitSpeakers } from "./speakers";

describe("폰 단체 대화: 말한 사람별로 나누기", () => {
	it("이름: 대사 줄마다 사람을 바꾸고, 이름 없는 줄은 앞 사람 말에 붙는다", () => {
		expect(splitSpeakers("유나: 안녕!\n오늘 뭐 해?\n카이: 나도 있어\n유나: 반가워", "유나")).toEqual([
			{ speaker: "유나", text: "안녕!\n오늘 뭐 해?" },
			{ speaker: "카이", text: "나도 있어" },
			{ speaker: "유나", text: "반가워" },
		]);
	});
	it("처음부터 이름이 없으면 대화 상대의 말", () => {
		expect(splitSpeakers("응, 알았어\n카이: 나도!", "세라피나")).toEqual([
			{ speaker: "세라피나", text: "응, 알았어" },
			{ speaker: "카이", text: "나도!" },
		]);
	});
	it("지문·대사 기호로 시작하는 줄은 이름으로 보지 않는다", () => {
		expect(splitSpeakers('*웃으며: 손을 흔든다*\n"안녕: 친구"', "유나")).toEqual([{ speaker: "유나", text: '*웃으며: 손을 흔든다*\n"안녕: 친구"' }]);
	});
});
