// Android·데스크톱 구글 드라이브 인증 보조 (CLAUDE.md 2-D, ADR-011, 1-J-3).
// - 모바일은 Play 서비스가 access token을 주고, 연결 해제·401 발생 시 GoogleAuthUtil.clearToken으로 캐시를 비운 뒤 동의 화면을 다시 띄운다.
// - 권한이 끊겼거나 만료된 상태에서 동기화·데이터 삭제를 누르면 에러 메시지를 반환하지 않고 즉시 권한을 다시 받아 이어서 실행한다.
import { get } from "svelte/store";
import { _ } from "svelte-i18n";
import type { DriveStatus } from "$ipc/bindings";
import { AppError, call, commands } from "$shared/ipc/client";

const t = (k: string, values?: Record<string, string | number>) => get(_)(k, values ? { values } : undefined);

type Native = {
	postMessage(msg: string): void;
	addEventListener?(type: "message", fn: (e: { data: string }) => void): void;
	onmessage?: ((e: { data: string }) => void) | null;
};
const native = () => (window as unknown as { SandcastleNative?: Native }).SandcastleNative;
const net = () => (window as unknown as { SandcastleNet?: { unmetered(): boolean } }).SandcastleNet;

export const hasNativeDrive = () => !!native() && !!net();

let seq = 0;
let listening = false;
let lastAndroidToken = "";
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

export class NativeDriveError extends Error {
	readonly code: string;
	constructor(code: string, message: string) {
		super(message);
		this.name = "NativeDriveError";
		this.code = code;
	}
}

/** 백엔드 오류가 드라이브 재인증 필요(`driveAuthRequired: true`)인지 확인한다 */
export function isDriveAuthRequired(err: unknown): boolean {
	if (err instanceof NativeDriveError && err.code === "needs_consent") return true;
	const details = (err as AppError | { details?: unknown } | null | undefined)?.details;
	return Boolean(details && typeof details === "object" && (details as { driveAuthRequired?: boolean }).driveAuthRequired === true);
}

/** Play 서비스가 캐시해 둔 취소·만료 토큰을 비운다 */
export function clearNativeDriveToken(): Promise<void> {
	const n = native();
	const tok = lastAndroidToken;
	lastAndroidToken = "";
	if (!n) return Promise.resolve();
	listen(n);
	const id = String(++seq);
	return new Promise((resolve) => {
		const timer = setTimeout(() => {
			waiting.delete(id);
			resolve();
		}, 5_000);
		waiting.set(id, () => {
			clearTimeout(timer);
			resolve();
		});
		n.postMessage(`drive:clear:${id}:${tok}`);
	});
}

/** 폰에 토큰을 묻는다. interactive면 처음이거나 권한이 취소됐을 때 구글 동의 화면을 띄운다 */
export function nativeDriveToken(interactive: boolean): Promise<string> {
	const n = native();
	if (!n) return Promise.reject(new NativeDriveError("unavailable", t("androidDrive.unavailable")));
	listen(n);
	const id = String(++seq);
	return new Promise((resolve, reject) => {
		const timer = setTimeout(() => {
			waiting.delete(id);
			reject(new NativeDriveError("timeout", t("androidDrive.noResponse")));
		}, 120_000);
		waiting.set(id, (r) => {
			clearTimeout(timer);
			if (r.token) {
				lastAndroidToken = r.token;
				resolve(r.token);
			} else {
				const code = r.error ?? "unknown";
				reject(new NativeDriveError(code, explain(r.error)));
			}
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

/**
 * 폰에서 토큰을 받아 백엔드에 넣는다.
 * - allowConsent(또는 connect)가 true일 때 조용한 토큰 조회가 `needs_consent`로 돌아오거나 백엔드 검증에서 401(`driveAuthRequired`)이 나면,
 *   에러를 반환하지 않고 캐시를 비운 뒤 구글 동의 화면을 바로 띄워 새 권한을 받는다 (1-J-3).
 */
export async function prepareDrive(connect: boolean, allowConsent = connect): Promise<DriveStatus> {
	let token: string;
	try {
		token = await nativeDriveToken(connect);
	} catch (e) {
		if (allowConsent && e instanceof NativeDriveError && e.code === "needs_consent") {
			token = await nativeDriveToken(true);
			connect = true;
		} else {
			throw e;
		}
	}
	try {
		return await call(commands.driveUseToken(token, connect), { source: "sync", silent: allowConsent });
	} catch (e) {
		if (allowConsent && isDriveAuthRequired(e)) {
			await clearNativeDriveToken();
			const fresh = await nativeDriveToken(true);
			return call(commands.driveUseToken(fresh, true), { source: "sync" });
		}
		throw e;
	}
}

/**
 * 구글 드라이브 작업을 실행하되, 권한이 끊겼거나 만료(`driveAuthRequired`)되었으면
 * 에러 토스트를 띄우지 않고 즉시 다시 권한을 받아 재실행한다 (Android·데스크톱 공통, 1-J-3).
 */
export async function runWithDriveAuth<T>(
	isDriveTarget: boolean,
	run: (silentFirstAttempt: boolean) => Promise<T>,
	opts: { allowConsent?: boolean; onReconnected?: (s: DriveStatus) => void } = {},
): Promise<T> {
	const allowConsent = opts.allowConsent ?? true;
	const phone = hasNativeDrive();
	if (!isDriveTarget) {
		return run(false);
	}
	if (phone) {
		await prepareDrive(false, allowConsent);
	}
	try {
		return await run(allowConsent);
	} catch (e) {
		if (!allowConsent || !isDriveAuthRequired(e)) {
			throw e;
		}
		if (phone) {
			await clearNativeDriveToken();
			const status = await prepareDrive(true, true);
			opts.onReconnected?.(status);
		} else {
			const status = await call(commands.driveLogin(), { source: "sync" });
			opts.onReconnected?.(status);
		}
		return run(false);
	}
}

/** 지금 연결이 데이터 요금이 들지 않는가 (모르면 true) */
export function unmetered(): boolean {
	try {
		return net()?.unmetered() ?? true;
	} catch {
		return true;
	}
}
