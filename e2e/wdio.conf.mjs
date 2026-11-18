// E2E 스모크 (CLAUDE.md 1-D-1): 실제 앱(디버그 빌드)을 띄워 msedgedriver(Windows) 또는 tauri-driver로 붙어 WebdriverIO로 조작한다.
//
// 준비 (한 번): WebView2와 같은 버전의 msedgedriver (Windows 밖에서는 cargo install tauri-driver --locked도)
// 실행:        pnpm e2e:build && pnpm e2e
//   - 앱은 identifier를 com.dilly311.sandcastle.e2e로 바꿔 빌드한다 → 평소 앱 데이터와 섞이지 않음 (시작 전에 비움)
//   - AI는 이 컴퓨터에서 띄운 가짜 공급자만 부른다 (요금 없음)
//   - msedgedriver 위치: MSEDGEDRIVER 환경 변수, 없으면 PATH
//   - Windows: 앱을 직접 띄우고 msedgedriver를 앱의 원격 디버깅 포트에 붙인다(debuggerAddress).
//     tauri-driver 방식은 드라이버가 환경 변수(WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS)로 포트를 여는데,
//     GitHub 러너의 WebView2(156)가 그 환경 변수를 무시해 세션이 안 만들어졌다(DevToolsActivePort).
//     포트는 E2E 빌드에만 있는 설정(tauri.e2e.conf.json의 additionalBrowserArgs)으로 연다
import { spawn, spawnSync } from "node:child_process";
import http from "node:http";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const exe = path.join(root, "src-tauri", "target", "debug", process.platform === "win32" ? "sandcastle.exe" : "sandcastle");
export const E2E_IDENTIFIER = "com.dilly311.sandcastle.e2e";

/** E2E 빌드의 앱 데이터 폴더 (Tauri의 app_data_dir와 같은 규칙) */
function e2eDataDir() {
	if (process.platform === "win32") return path.join(process.env.APPDATA ?? path.join(os.homedir(), "AppData", "Roaming"), E2E_IDENTIFIER);
	if (process.platform === "darwin") return path.join(os.homedir(), "Library", "Application Support", E2E_IDENTIFIER);
	return path.join(process.env.XDG_DATA_HOME ?? path.join(os.homedir(), ".local", "share"), E2E_IDENTIFIER);
}

const DEBUG_PORT = 9333; // tauri.e2e.conf.json과 같게
const attach = process.platform === "win32";

let driver;
let app;

/** 주소가 응답할 때까지(그리고 ready(본문)가 참일 때까지) 기다림 */
async function waitFor(url, ms, what, ready = () => true) {
	const end = Date.now() + ms;
	while (Date.now() < end) {
		const ok = await new Promise((done) => {
			const req = http.get(url, (res) => {
				let body = "";
				res.setEncoding("utf8");
				res.on("data", (c) => (body += c));
				res.on("end", () => done(res.statusCode < 500 && ready(body)));
			});
			req.on("error", () => done(false));
			req.setTimeout(2_000, () => req.destroy());
		});
		if (ok) return;
		await new Promise((r) => setTimeout(r, 500));
	}
	throw new Error(`${what}이(가) ${ms / 1000}초 안에 뜨지 않음: ${url}`);
}

/** 자식 프로세스까지 끝냄 (.cmd 감싸기는 cmd.exe 밑에 msedgedriver가 있고, 앱 밑에는 WebView2가 있다) */
function stop(p) {
	if (!p || p.exitCode !== null) return;
	if (process.platform === "win32") spawnSync("taskkill", ["/pid", String(p.pid), "/t", "/f"], { stdio: "ignore" });
	else p.kill();
}

function startDriver() {
	const native = process.env.MSEDGEDRIVER || "msedgedriver";
	// .cmd는 셸로만 띄울 수 있다
	const shell = native.toLowerCase().endsWith(".cmd");
	return spawn(shell ? `"${native}"` : native, ["--port=4444"], { shell, stdio: [null, process.stdout, process.stderr] });
}

export const config = {
	runner: "local",
	specs: ["./specs/**/*.e2e.mjs"],
	maxInstances: 1,
	hostname: "127.0.0.1",
	port: 4444,
	capabilities: [
		attach
			? {
					maxInstances: 1,
					browserName: "webview2",
					"ms:edgeOptions": { debuggerAddress: `127.0.0.1:${DEBUG_PORT}` },
					// 이름이 브라우저라 wdio가 BiDi로 바꾸면 스크립트가 다른 곳에서 돌아 IPC(Origin)·화면 확인이 틀어진다
					"wdio:enforceWebDriverClassic": true,
				}
			: { maxInstances: 1, "tauri:options": { application: exe } },
	],
	logLevel: "warn",
	framework: "mocha",
	reporters: ["spec"],
	mochaOpts: { ui: "bdd", timeout: 120_000 },
	waitforTimeout: 20_000,

	onPrepare() {
		if (!fs.existsSync(exe)) throw new Error(`앱이 없음: ${exe}\n먼저 pnpm e2e:build`);
		// 지난 E2E의 데이터를 지우고 깨끗하게 시작 (E2E 전용 폴더만)
		const dir = e2eDataDir();
		if (dir.endsWith(E2E_IDENTIFIER)) fs.rmSync(dir, { recursive: true, force: true });
	},
	async beforeSession() {
		if (attach) {
			app = spawn(exe, [], { stdio: [null, process.stdout, process.stderr] });
			// 포트가 열린 직후의 페이지는 아직 about:blank라, 거기 붙으면 IPC가 Origin 없음으로 거절된다 → 앱 주소가 뜰 때까지
			await waitFor(`http://127.0.0.1:${DEBUG_PORT}/json/list`, 60_000, "앱 화면(원격 디버깅 포트)", (b) => b.includes("tauri.localhost"));
			driver = startDriver();
			await waitFor("http://127.0.0.1:4444/status", 30_000, "msedgedriver");
			return;
		}
		const args = process.env.MSEDGEDRIVER ? ["--native-driver", process.env.MSEDGEDRIVER] : [];
		const bin = path.join(os.homedir(), ".cargo", "bin", process.platform === "win32" ? "tauri-driver.exe" : "tauri-driver");
		driver = spawn(fs.existsSync(bin) ? bin : "tauri-driver", args, { stdio: [null, process.stdout, process.stderr] });
	},
	afterSession() {
		stop(driver);
		stop(app);
	},
};
