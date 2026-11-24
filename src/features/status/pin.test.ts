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

	it("같은 출처가 한 메시지에 여럿이면 모두 반환 (g 플래그 정규식)", () => {
		const r1 = (n: number) => `<div data-sc-pin="status:r1">HP ${n}</div>`;
		const messages = [msg("a", "assistant", `${r1(10)}\n\n${r1(20)}`)];
		const blocks = latestStatusBlocks(messages, {}, true);
		expect(blocks).toHaveLength(2);
		expect(blocks[0].code).toContain("HP 10");
		expect(blocks[1].code).toContain("HP 20");
	});

	it("allowedKeys 필터: 선택된 정규식 출처만 반환", () => {
		const hp = (n: number) => `<div data-sc-pin="status:r-hp">HP ${n}</div>`;
		const mp = (n: number) => `<div data-sc-pin="status:r-mp">MP ${n}</div>`;
		const messages = [msg("a", "assistant", `${hp(10)}\n\n${mp(20)}`)];
		const allowed = new Set(["status:r-hp"]);
		const blocks = latestStatusBlocks(messages, {}, true, allowed);
		expect(blocks).toHaveLength(1);
		expect(blocks[0].code).toContain("HP 10");
	});

	it("상태창 정규식이 ```html 코드 블록이나 여러 줄 순수 HTML을 반환해도 본문에서 숨기고 상태창으로 보낸다", () => {
		const fencedPin = `<div data-sc-pin="status:r-iframe">\`\`\`html\n<style>.box{color:red}</style><div class="box">HP 100</div>\n\`\`\`</div><!--/sc-pin-->`;
		const multilineHtmlPin = `<div data-sc-pin="status:r-pure"><div\n  style="padding: 8px;"\n>\n  <div>MP 50</div>\n\n  <div>SP 30</div>\n</div></div><!--/sc-pin-->`;
		const text = `대사 시작\n\n${fencedPin}\n\n중간 지문\n\n${multilineHtmlPin}\n\n대사 끝`;

		const msgBlocks = toBlocks(text, { safeHtml: true });
		const visible = msgBlocks.filter((b) => !isStatusPin(b));
		expect(visible).toHaveLength(3);

		const allowed = new Set(["status:r-iframe", "status:r-pure"]);
		const status = latestStatusBlocks([msg("a", "assistant", text)], {}, true, allowed);
		expect(status).toHaveLength(2);
		expect(status[0]).toEqual({ kind: "html", code: '<style>.box{color:red}</style><div class="box">HP 100</div>', pin: "status:r-iframe" });
		expect(status[1].scripts).toBe(false);
		expect(status[1].code).toContain("MP 50");
		expect(status[1].code).toContain("SP 30");
	});

	it("앞 상태창 HTML에 닫히지 않은 div가 있어도 뒤 상태창과 본문을 삼키지 않는다", () => {
		const broken1 = `<div data-sc-pin="status:r1"><div class="unclosed">첫째 상태창</div><!--/sc-pin-->`;
		const broken2 = `<div data-sc-pin="status:r2"><div>둘째 상태창</div></div><!--/sc-pin-->`;
		const text = `${broken1}\n\n본문 유지\n\n${broken2}`;
		const allowed = new Set(["status:r1", "status:r2"]);
		const status = latestStatusBlocks([msg("a", "assistant", text)], {}, true, allowed);
		expect(status).toHaveLength(2);
		expect(status[0].code).toContain("첫째 상태창");
		expect(status[1].code).toContain("둘째 상태창");
		const visible = toBlocks(text, { safeHtml: true }).filter((b) => !isStatusPin(b));
		expect(visible.map((b) => (b.kind === "md" ? b.html : "")).join("")).toContain("본문 유지");
	});
});
