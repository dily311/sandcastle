// Android 구글 드라이브 (CLAUDE.md 2-D, ADR-011). 폰의 Play 서비스가 권한(drive.appdata)을 주고, 이 파일이 받은
// access token을 백엔드에 넘긴다(메모리에만). refresh token은 없다 — 동기화할 때마다 폰에 다시 묻는다(이미 허락했으면 화면 없이).
// 앱(MainActivity)이 앱 주소에만 SandcastleNative·SandcastleNet을 넣어 주므로 데스크톱·브라우저에서는 hasNativeDrive()가 false
import { get } from "svelte/store";
import { _ } from "svelte-i18n";
import { call, commands } from "$shared/ipc/client";

const t = (k: string, values?: Record<string, string | number>) => get(_)(k, values ? { values } : undefined);

type Native = { postMessage(msg: string): void; addEventListener?(type: "message", fn: (e: { data: string }) => void): void; onmessage?: ((e: { data: string }) => void) | null };
const native = () => (window as unknown as { SandcastleNative?: Native }).SandcastleNative;
const net = () => (window as unknown as { SandcastleNet?: { unmetered(): boolean } }).SandcastleNet;

export const hasNativeDrive = () => !!native() && !!net();

let seq = 0;
let listening = false;
const waiting = new Map<string, (r: { token?: string; error?: string }) => void>();

function listen(n: Native) {
	if (listening) return;
	listening = true;
	const onMsg = (e: { data: string }) => {
		let d: { kind?: string; id?: string; token?: string; error?: string };
		try {
			d = JSON.parse(e.data);
		} catch {
			return;
		}
		if (d.kind !== "drive" || !d.id) return;
		waiting.get(d.id)?.(d);
		waiting.delete(d.id);
	};
	if (n.addEventListener) n.addEventListener("message", onMsg);
	else n.onmessage = onMsg;
}

/** 폰에 토큰을 묻는다. interactive면 처음일 때 구글 동의 화면을 띄운다 */
export function nativeDriveToken(interactive: boolean): Promise<string> {
	const n = native();
	if (!n) return Promise.reject(new Error(t("androidDrive.unavailable")));
	listen(n);
	const id = String(++seq);
	return new Promise((resolve, reject) => {
		const timer = setTimeout(() => {
			waiting.delete(id);
			reject(new Error(t("androidDrive.noResponse")));
		}, 120_000);
		waiting.set(id, (r) => {
			clearTimeout(timer);
			if (r.token) resolve(r.token);
			else reject(new Error(explain(r.error)));
		});
		n.postMessage(`drive:authorize:${id}:${interactive ? 1 : 0}`);
	});
}

function explain(code?: string) {
	if (code === "cancelled") return t("androidDrive.cancelled");
	if (code === "needs_consent") return t("androidDrive.needsConsent");
	// 10 = DEVELOPER_ERROR: 구글 클라우드의 Android 클라이언트(패키지 이름·서명 키 SHA-1)가 이 앱과 다름
	if (code === "api:10") return t("androidDrive.unknownApp");
	if (code?.startsWith("api:7")) return t("androidDrive.networkError");
	return t("androidDrive.permFailed", { code: code ?? t("androidDrive.unknownCode") });
}

/** 폰에서 토큰을 받아 백엔드에 넣는다. connect=처음 연결(동의 화면 허용, 계정을 읽고 저장소를 드라이브로) */
export async function prepareDrive(connect: boolean) {
	const token = await nativeDriveToken(connect);
	return call(commands.driveUseToken(token, connect), { source: "sync" });
}

/** 지금 연결이 데이터 요금이 들지 않는가 (모르면 true) */
export function unmetered(): boolean {
	try {
		return net()?.unmetered() ?? true;
	} catch {
		return true;
	}
}
