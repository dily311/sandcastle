# AI Game Chat — scalable MVP

Stack: Tauri 2 + Svelte 5 + Vite + TypeScript + Pure CSS + PixiJS 8 + Rust/reqwest.

## Product boundary
- Local-first desktop/mobile application.
- No application server.
- Rust owns provider HTTP calls so API keys do not live in the Svelte/WebView networking layer.
- Chat mode is the primary product.
- Game mode is a separate runtime that can consume the same conversation/state layer.
- **View more details**: [Tech spec](./docs/Tech%20Spec.md)

## 현재 상태
- 저장소: SQLite(sqlx) + 파일 기반 마이그레이션 (`src-tauri/src/db/migrations`)
- API 키: OS 보안 저장소(keyring). 프로바이더 HTTP 호출은 Rust 어댑터(OpenAI 호환, Claude, Gemini, Vertex AI, NovelAI)
- 채팅 생성은 작업(Task)으로 실행되어 대화를 오가도 이어지고, 앱을 다시 켜면 중단된 생성을 복구할 수 있다
- 게임 모드: 이벤트 구동 엔진, .tgame 패키지 가져오기/내보내기, 책갈피 분기
- 남은 일과 보류한 결정은 `docs/`의 ADR 참고

## 실행
```bash
pnpm install
pnpm tauri:dev
```

Android는 Tauri Android 타깃을 초기화한 뒤 `pnpm android:dev`로 실행한다. 링커 설정은 `src-tauri/.cargo/config.toml`.

## 검사
```bash
pnpm check        # Svelte·TS 타입 검사
pnpm test:run     # 프론트 단위 테스트 (Vitest)
cd src-tauri && cargo test && cargo clippy --all-targets
```

프론트 바인딩(`src/ipc/bindings.ts`)은 Rust 커맨드에서 생성된다. 커맨드를 바꾸면 `cargo run --bin export-bindings`로 다시 만든다. CI(`.github/workflows/ci.yml`)가 위 검사와 바인딩 최신 여부를 확인한다.

## License
This project is licensed under the **GNU Affero General Public License v3.0 (AGPL-3.0)** - see the [LICENSE](LICENSE) file for details.

요약: 개조 재배포 다 가능. 출처와 소스코드 공개 필수
다만 개조 재배포 시 문제 책임 소재는 해당 프로젝트에 없음
개조버전 오류에 대한 문의의 경우 이곳에 남기지 말것
