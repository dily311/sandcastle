// 앱 버전은 한 곳(package.json)에서 (CLAUDE.md 1-E-1).
//   node scripts/version.mjs            → 세 곳이 같은지 확인 (CI)
//   node scripts/version.mjs 0.2.0      → package.json · src-tauri/Cargo.toml을 바꾼다 (tauri.conf.json은 package.json을 읽음)
// 버전을 올리면 CHANGELOG.md에 그 버전 칸을 적고, 태그 v<버전>을 달면 릴리스 워크플로가 설치 파일을 만든다
import fs from "node:fs";

const SEMVER = /^\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?$/;
const pkgPath = "package.json";
const cargoPath = "src-tauri/Cargo.toml";
const confPath = "src-tauri/tauri.conf.json";

const pkg = JSON.parse(fs.readFileSync(pkgPath, "utf8"));
const cargo = fs.readFileSync(cargoPath, "utf8");
const conf = JSON.parse(fs.readFileSync(confPath, "utf8"));
const cargoVersion = () => /^\[package\][\s\S]*?^version\s*=\s*"([^"]+)"/m.exec(fs.readFileSync(cargoPath, "utf8"))?.[1];

const next = process.argv[2];
if (next) {
	if (!SEMVER.test(next)) throw new Error(`버전 형식이 아님: ${next} (예: 0.2.0)`);
	pkg.version = next;
	fs.writeFileSync(pkgPath, JSON.stringify(pkg, null, 2) + "\n");
	fs.writeFileSync(cargoPath, cargo.replace(/^(\[package\][\s\S]*?^version\s*=\s*)"[^"]+"/m, `$1"${next}"`));
	console.log(`버전 ${next} — CHANGELOG.md에 적고, cargo가 Cargo.lock을 고치게 한 번 빌드(또는 cargo check)할 것`);
}

const versions = { "package.json": pkg.version, "Cargo.toml": cargoVersion(), "tauri.conf.json": conf.version === "../package.json" ? pkg.version : conf.version };
const all = new Set(Object.values(versions));
if (all.size !== 1 || conf.version !== "../package.json") {
	console.error("버전이 맞지 않음:", versions, conf.version !== "../package.json" ? "(tauri.conf.json의 version은 \"../package.json\"이어야 함)" : "");
	process.exit(1);
}
console.log(`버전 ${pkg.version} (package.json · Cargo.toml · tauri.conf.json)`);
