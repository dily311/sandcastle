# 테마 CSS 클래스 (5-E-17-2)

사용자 테마(설정 > 테마 > CSS 스니펫, 테마 파일의 `css`)에서 고를 수 있게 화면의 주요 요소에 붙인 이름이다.
앱 안쪽 클래스(`.block`, `.who` 등)는 바뀔 수 있지만, 아래 `sc-` 클래스는 이름을 지킨다(바꾸면 이 문서에 적고 예전 이름을 한동안 같이 붙인다).

## 쓰는 법

- 스니펫은 앱 스타일 **뒤에** 들어간다. 앱 스타일과 세기가 같으면 스니펫이 이긴다.
- 앱 스타일을 확실히 덮으려면 선택자 앞에 `.sc-app `을 붙인다. 예: `.sc-app .sc-msg-ai { … }`
- 화면마다 다르게: `.sc-app[data-page="character"]`, `"phone"`, `"home"`, `"game"`, `"settings"` 등.
- 색은 되도록 테마 변수(`--accent`, `--text`, `--surface-2` …)로 — 밝은/어두운 모드를 같이 따라간다.

```css
/* 예: AI 메시지를 카드처럼, 대사는 강조색 */
.sc-app .sc-msg-ai { background: var(--surface-2); border-radius: 12px; padding: 8px 12px; }
.sc-app .sc-md q { color: var(--accent); }
/* 폰: 내 말풍선 색 */
.sc-app .sc-bubble-me .sc-bubble { background: #ffd84d; }
/* 타이핑 표시 그림을 크게 (그림은 설정 > 대화 > 타이핑 표시 > 그림·GIF) */
.sc-app .sc-typing { --typing-image-size: 48px; }
```

## 앱 틀

| 클래스 | 요소 |
| --- | --- |
| `sc-app` | 앱 전체. `data-page`에 지금 화면 이름 |
| `sc-titlebar` | 맨 위 줄 |
| `sc-sidebar` | 왼쪽 메뉴 |
| `sc-main` | 가운데 화면 |
| `sc-side-panel` | 오른쪽 패널 |
| `sc-avatar` | 모든 프로필 그림(동그라미) |

## 대화 (캐릭터)

| 클래스 | 요소 |
| --- | --- |
| `sc-conv-list` | 대화 목록 |
| `sc-chat` | 대화 화면 전체 |
| `sc-chat-scroll` | 메시지가 스크롤되는 칸 |
| `sc-chat-log` | 메시지들을 감싼 본문 |
| `sc-chapter` | 챕터 제목 |
| `sc-msg` | 메시지 하나 |
| `sc-msg-ai` · `sc-msg-user` | AI 메시지 · 내 메시지 |
| `sc-msg-live` | 만드는 중인 메시지 |
| `sc-msg-hidden` | AI에게 숨긴 메시지 |
| `sc-msg-head` | 메시지 머리줄(번호·이름·시각) |
| `sc-msg-no` · `sc-msg-name` | 번호 · 이름 |
| `sc-msg-actions` | 메시지 동작 버튼 줄 |
| `sc-md` | 메시지 본문(마크다운). 안의 `em` = 지문, `q` = 대사, `strong`, `table`, `blockquote` 등 |
| `sc-ui-html` | 본문 안 HTML UI(`<details>` 상태창 등) |
| `sc-speaker` | 다인봇 `이름: 대사` 줄의 말한 사람 |
| `sc-cg` | 본문 안 캐릭터 에셋 그림 |
| `sc-typing` · `sc-typing-text` · `sc-typing-image` | 응답 기다리는 표시 · 그 글 · 고른 그림 (`data-style`: `blink`·`bounce`·`none`·`image`, 점은 `.dots span`, 그림 높이는 `--typing-image-size`, 기본 32px) |
| `sc-composer` | 아래 입력 영역 |
| `sc-input` | 메시지 입력칸 |
| `sc-send-button` | 보내기 버튼 |

## 폰

| 클래스 | 요소 |
| --- | --- |
| `sc-phone` | 폰 화면 전체 |
| `sc-phone-room` | 대화방 |
| `sc-phone-room-bar` | 대화방 위 줄(이름) |
| `sc-phone-msgs` | 말풍선이 스크롤되는 칸 |
| `sc-bubble-group` | 같은 사람의 말풍선 묶음 (`sc-bubble-me` · `sc-bubble-them`) |
| `sc-bubble` | 말풍선 하나 |
| `sc-bubble-name` | 상대 이름 |
| `sc-phone-quick-reply` | 입력 줄 위 빠른 답장 버튼 줄 (슬래시 코어 플러그인이 켜졌을 때) |
| `sc-phone-input` | 입력 줄 |

## 홈

| 클래스 | 요소 |
| --- | --- |
| `sc-home` | 홈 화면 전체 (홈 전용 CSS는 설정 > 홈에도 있음) |
