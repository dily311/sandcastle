// E2E 스모크 (CLAUDE.md 1-D-1): 실제 앱(디버그 빌드)을 tauri-driver로 띄워 WebdriverIO로 조작한다.
//
// 준비 (한 번): cargo install tauri-driver --locked, WebView2와 같은 버전의 msedgedriver
// 실행:        pnpm e2e:build && pnpm e2e
//   - 앱은 identifier를 com.dilly311.sandcastle.e2e로 바꿔 빌드한다 → 평소 앱 데이터와 섞이지 않음 (시작 전에 비움)
//   - AI는 이 컴퓨터에서 띄운 가짜 공급자만 부른다 (요금 없음)
//   - msedgedriver 위치: MSEDGEDRIVER 환경 변수, 없으면 PATH
import { spawn } from "node:child_process";
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

let driver;

export const config = {
	runner: "local",
	specs: ["./specs/**/*.e2e.mjs"],
	maxInstances: 1,
	hostname: "127.0.0.1",
	port: 4444,
	capabilities: [{ maxInstances: 1, "tauri:options": { application: exe } }],
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
	beforeSession() {
		const args = process.env.MSEDGEDRIVER ? ["--native-driver", process.env.MSEDGEDRIVER] : [];
		const bin = path.join(os.homedir(), ".cargo", "bin", process.platform === "win32" ? "tauri-driver.exe" : "tauri-driver");
		driver = spawn(fs.existsSync(bin) ? bin : "tauri-driver", args, { stdio: [null, process.stdout, process.stderr] });
	},
	afterSession() {
		driver?.kill();
	},
};
