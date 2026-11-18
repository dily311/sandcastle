import { describe, expect, it } from "vitest";
import { describeSrc } from "./imageViewer.svelte";

describe("describeSrc", () => {
	it("convertFileSrc 주소에서 파일 이름과 디스크 경로를 뽑는다", () => {
		const r = describeSrc("http://asset.localhost/C%3A%5CUsers%5Ckim%5Cgenerated%5C%EA%B7%B8%EB%A6%BC.png");
		expect(r.name).toBe("그림.png");
		expect(r.path).toBe("C:\\Users\\kim\\generated\\그림.png");
	});
	it("인터넷 그림은 확장자가 있을 때만 이름을 쓰고 경로는 없다", () => {
		expect(describeSrc("https://example.com/a/cat.webp")).toEqual({ name: "cat.webp", path: null });
		expect(describeSrc("https://placehold.co/832x1216/png?text=forest")).toEqual({ name: "", path: null });
	});
	it("data·blob 그림은 이름도 경로도 없다", () => {
		expect(describeSrc("data:image/png;base64,AAAA")).toEqual({ name: "", path: null });
		expect(describeSrc("blob:http://localhost/abc")).toEqual({ name: "", path: null });
	});
});
