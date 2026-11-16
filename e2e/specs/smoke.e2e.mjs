// E2E 스모크 (CLAUDE.md 1-D-1): 앱 시작 → 캐릭터 가져오기 → 가짜 공급자로 메시지 한 번 → 백업
// 파일 고르기는 운영체제 창이라 조작할 수 없어, 가져오기·백업은 같은 백엔드 커맨드를 앱 안에서 부른다
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { browser, $, expect } from "@wdio/globals";
import { startFakeOpenAI, REPLY } from "../fake-openai.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..");
const card = path.join(root, "src-tauri", "tests", "sillyTavern", "character", "default_Seraphina.png");

/** 앱 안에서 백엔드 커맨드 부르기 (화면이 쓰는 것과 같은 IPC) */
function invoke(cmd, args = {}) {
	return browser.executeAsync(
		(cmd, args, done) => {
			window.__TAURI_INTERNALS__.invoke(cmd, args).then(
				(v) => done({ ok: v }),
				(e) => done({ err: e }),
			);
		},
		cmd,
		args,
	);
}
async function must(cmd, args) {
	const r = await invoke(cmd, args);
	if (r.err) throw new Error(`${cmd} 실패: ${JSON.stringify(r.err)}`);
	return r.ok;
}
const byText = (tag, text) => $(`//${tag}[contains(normalize-space(.), "${text}")]`);

describe("스모크", () => {
	let fake;
	let profileId;
	const started = Date.now();

	before(async () => {
		fake = await startFakeOpenAI();
	});
	after(async () => {
		// API 키는 OS 자격 증명 관리자에 들어가므로 프로필을 지워 키도 지운다
		if (profileId) await invoke("delete_api_profile", { id: profileId });
		await fake?.close();
	});

	it("앱이 시작한다", async () => {
		await expect(byText("h1", "캐릭터")).toBeDisplayed({ wait: 30_000 });
		console.log(`[e2e] 시작해서 캐릭터 화면까지: ${Date.now() - started}ms (드라이버·창 띄우기 포함)`);
	});

	it("캐릭터를 가져온다", async () => {
		const r = await must("import_sillytavern", { path: card, name: null, characterId: null, presetId: null });
		expect(r.kind).toBe("character");
		await browser.refresh();
		await expect(byText("b", "Seraphina")).toBeDisplayed({ wait: 20_000 });
	});

	it("가짜 공급자로 메시지를 주고받는다", async () => {
		const p = await must("create_api_profile", {
			input: { name: "E2E", provider: "openai", formatType: "openai", baseUrl: fake.url, model: "fake-model" },
			apiKey: "sk-e2e-fake-key",
		});
		profileId = p.id;
		await must("update_app_setting", { patch: { selectedApiProfile: p.id } });

		await byText("b", "Seraphina").click();
		await byText("button", "대화 시작").click();
		const box = await $("textarea[aria-label='메시지']");
		await box.waitForDisplayed({ timeout: 20_000 });
		await box.setValue("안녕, E2E 점검이야");
		await $("button[aria-label='보내기']").click();
		await expect(byText("div", REPLY)).toBeDisplayed({ wait: 30_000 });
		expect(fake.calls.some((c) => c.url.endsWith("/chat/completions") && c.auth === "Bearer sk-e2e-fake-key")).toBe(true);
	});

	it("백업 파일을 만든다", async () => {
		const out = path.join(os.tmpdir(), `sandcastle-e2e-${Date.now()}.scbackup`);
		const r = await must("export_backup", { path: out });
		expect(fs.existsSync(out)).toBe(true);
		expect(r.totals.messages).toBeGreaterThanOrEqual(2);
		// 키는 백업에 없어야 한다 (1-B-4와 같은 확인을 실제 앱으로)
		expect(fs.readFileSync(out).includes("sk-e2e-fake-key")).toBe(false);
		fs.rmSync(out, { force: true });
	});
});
