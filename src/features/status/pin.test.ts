import { describe, expect, it } from "vitest";
import { isStatusPin, latestStatusBlock, latestStatusBlocks, pinKey } from "./pin";
import { toBlocks } from "$features/character/format";

const STATUS = (hp: number) => `<div data-sc-pin="status">HP ${hp}</div>`;
const msg = (id: string, role: string, content: string) => ({ id, role, content });

describe("상태창 고정 (5-F-4-2)", () => {
	it("표시가 있는 HTML 블록만 상태창", () => {
		expect(isStatusPin({ kind: "html", code: STATUS(1) })).toBe(true);
		expect(isStatusPin({ kind: "html", code: "<div data-sc-pin='status'>x</div>" })).toBe(true);
		expect(isStatusPin({ kind: "html", code: "<div>그냥 HTML</div>" })).toBe(false);
		expect(isStatusPin({ kind: "md", html: STATUS(1) })).toBe(false);
	});

	it("가장 최근 AI 메시지의 것 하나 — 사용자 메시지는 보지 않고, 보이는 글(정규식 결과)을 쓴다", () => {
		const messages = [
			msg("a", "assistant", `첫 장면\n\n${STATUS(10)}`),
			msg("b", "user", STATUS(99)),
			msg("c", "assistant", "상태창 없는 답"),
			msg("d", "assistant", "원문에는 없음"),
		];
		// d는 정규식이 상태창을 만든 경우
		const display = { d: `둘째 장면\n\n\`\`\`html\n${STATUS(7)}\n\`\`\`` };
		const b = latestStatusBlock(messages, display, true);
		expect(b?.code).toContain("HP 7");
		// 정규식 결과가 없으면 그 앞 AI 메시지의 것
		expect(latestStatusBlock(messages, {}, true)?.code).toContain("HP 10");
		expect(latestStatusBlock([msg("x", "assistant", "없음")], {}, true)).toBeNull();
	});

	it("문단 가운데 있는 상태창도 제 블록으로 떼어 낸다 (안에 같은 태그가 겹쳐도 닫는 태그까지)", () => {
		const text = '숲길을 걷는다. <div data-sc-pin="status"><div>HP 7</div><div>호감 4</div></div> 그리고 멈춘다.';
		const blocks = toBlocks(text, { safeHtml: true });
		const pinned = blocks.filter(isStatusPin);
		expect(pinned).toHaveLength(1);
		expect(pinned[0].code).toBe('<div data-sc-pin="status"><div>HP 7</div><div>호감 4</div></div>');
		// 앞뒤 글은 그대로 남는다
		const rest = blocks.filter((b) => !isStatusPin(b)).map((b) => (b.kind === "md" ? b.html : "")).join("");
		expect(rest).toContain("숲길을 걷는다.");
		expect(rest).toContain("그리고 멈춘다.");
		expect(latestStatusBlock([msg("a", "assistant", text)], {}, true)?.code).toContain("HP 7");
	});

	it("HTML 렌더링이 꺼져 있어도 상태창 요소는 거른 HTML 블록 (그 플러그인에 매이지 않게, 5-F-5-1)", () => {
		const text = '걷는다.\n\n<div data-sc-pin="status:r1"><b>HP</b> 7</div>\n\n<b>굵게</b> 그대로';
		const blocks = toBlocks(text, { safeHtml: false });
		const pinned = blocks.filter(isStatusPin);
		expect(pinned).toEqual([{ kind: "html", code: '<div data-sc-pin="status:r1"><b>HP</b> 7</div>', scripts: false }]);
		expect(latestStatusBlock([msg("a", "assistant", text)], {}, false)?.code).toContain("HP");
	});

	it("출처마다 가장 최근 것 하나씩, 출처 이름순 (5-F-5-2)", () => {
		const hp = (n: number) => `<div data-sc-pin="status:hp">HP ${n}</div>`;
		const inv = (s: string) => `<div data-sc-pin="status:bag">가방 ${s}</div>`;
		const messages = [msg("a", "assistant", `${hp(10)}\n\n${inv("칼")}`), msg("b", "assistant", hp(7))];
		const blocks = latestStatusBlocks(messages, {}, true);
		expect(blocks.map((b) => [pinKey(b), b.code])).toEqual([
			["status:bag", inv("칼")],
			["status:hp", hp(7)],
		]);
	});
});
