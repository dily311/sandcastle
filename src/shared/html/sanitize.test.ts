import { describe, expect, it } from "vitest";
import { htmlWithBreaks, sanitizeHtml } from "./sanitize";

describe("코드 블록 밖 HTML 거르기", () => {
	it("details·표 같은 UI는 남기고 스크립트·이벤트 속성은 뺀다", () => {
		const out = sanitizeHtml('<details open><summary onclick="x()">상태</summary><div class="hp" style="color:red">HP 80</div><script>alert(1)</script></details>');
		expect(out).toBe('<details open=""><summary>상태</summary><div class="hp" style="color:red">HP 80</div></details>');
	});
	it("iframe·폼·스타일 시트는 내용째, 모르는 태그는 껍데기만", () => {
		expect(sanitizeHtml('<iframe src="x"></iframe><form><input></form><style>body{}</style><marquee>흐름</marquee>')).toBe("흐름");
	});
	it("링크는 글자로, 그림은 https·data만, 위험한 CSS는 뺀다", () => {
		expect(sanitizeHtml('<a href="https://x.y" target="_blank">가기</a>')).toBe('<span title="https://x.y" class="html-link">가기</span>');
		expect(sanitizeHtml('<img src="javascript:x"><img src="https://a/b.png">')).toBe('<img loading="lazy"><img src="https://a/b.png" loading="lazy">');
		expect(sanitizeHtml('<div style="position: fixed; top:0; background:url(https://t)">덮기</div>')).toBe('<div style="top:0">덮기</div>');
	});
	it("줄바꿈은 <br>로, 블록 태그 앞뒤는 그대로", () => {
		expect(htmlWithBreaks("<details>\n<summary>상태</summary>\nHP 80\nMP 20\n</details>")).toBe("<details>\n<summary>상태</summary>\nHP 80<br>\nMP 20\n</details>");
	});
});
