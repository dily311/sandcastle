# 예제 플러그인

Sandcastle 커뮤니티 플러그인 예제입니다. 만드는 법은 [docs/플러그인 만들기.md](../../docs/플러그인%20만들기.md)에 있습니다.

| 폴더 | 보여 주는 것 | 권한 |
| --- | --- | --- |
| `word-count` | 오른쪽 패널, 명령, 설정칸, 대화 읽기, 자기 저장소 | `ui.panel` `ui.command` `data.read:chat` `storage` |
| `quote-check` | AI 응답 후처리 (메모와 자동 고침) | `messages.process` |
| `chat-summary` | AI 호출(명령 안에서만), 패널에 저장된 결과 보이기 | `ai` `ui.command` `ui.panel` `data.read:chat` `storage` |

## 설치해 보기

1. 설정 > 코어 플러그인에서 **커뮤니티 플러그인**을 켭니다.
2. 설정 > 커뮤니티 플러그인 > **폴더에서 설치**로 이 폴더 안의 예제 하나(예: `word-count`)를 고릅니다.
   zip으로 쓰려면 예제 폴더 *안의* 내용(`manifest.json`이 맨 위에 오게)을 zip으로 묶으세요.
3. 설치 미리보기에서 권한을 확인하고 설치합니다 (꺼진 채로 설치됩니다).
4. 목록의 스위치를 켜고 권한에 동의합니다.

이 폴더의 예제는 `cargo test`가 매번 설치 검사를 통과하는지 확인하므로, 예제를 고치면 manifest 규칙에 맞는지 바로 알 수 있습니다.
