# Contribution Guide

Meering Hero 프로젝트에 기여하기 위한 가이드.

## 개발 환경 준비

### 필수 도구

```bash
# Xcode Command Line Tools
xcode-select --install

# Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup target add aarch64-apple-darwin

# Node.js (LTS)
brew install node

# Tauri CLI
npm install -g @tauri-apps/cli
```

### 선택 도구

```bash
# Claude Code (요약 기능 테스트용)
npm install -g @anthropic-ai/claude-code
claude login

# create-dmg (DMG 번들링)
brew install create-dmg
```

### 프로젝트 클론 및 실행

```bash
git clone https://github.com/nhn-cloud/meering-hero.git
cd meering-hero

# 의존성 설치
npm install

# 개발 모드 실행
npm run tauri dev
```

첫 빌드 시 Rust 컴파일에 시간이 소요됩니다 (약 1-3분). 이후 증분 빌드는 수 초 내에 완료됩니다.

## 프로젝트 구조

```
meering-hero/
├── src/                       # React 프론트엔드
│   ├── components/            # UI 컴포넌트
│   ├── hooks/                 # React 커스텀 훅
│   └── lib/                   # 유틸리티 (Tauri IPC 래퍼)
├── src-tauri/                 # Rust 백엔드
│   ├── src/
│   │   ├── commands/          # Tauri IPC 커맨드
│   │   ├── audio/             # 오디오 캡처 + VAD
│   │   ├── stt/               # 음성 인식 파이프라인
│   │   ├── llm/               # Claude CLI 연동
│   │   ├── export/            # Dooray Wiki 클라이언트
│   │   ├── session/           # 세션 관리 + DB
│   │   └── model/             # 모델 다운로드/관리
│   ├── Cargo.toml
│   └── tauri.conf.json
├── docs/                      # 프로젝트 문서 (PRD, ADR)
├── userdocs/                  # 사용자/개발자 문서
└── .github/workflows/         # CI/CD
```

## 개발 워크플로우

### 브랜치 전략

```
main ← feature/xxx
     ← fix/xxx
     ← docs/xxx
```

- `main`: 릴리즈 브랜치 (태그 푸시 시 자동 빌드)
- `feature/*`: 기능 개발
- `fix/*`: 버그 수정
- `docs/*`: 문서 수정

### 커밋 메시지 컨벤션

```
feat: 새 기능 추가
fix: 버그 수정
docs: 문서 수정
refactor: 코드 리팩토링
style: 코드 포맷팅
test: 테스트 추가/수정
chore: 빌드/도구 변경
```

예시:
```
feat: Dooray Wiki 페이지 트리 네비게이션 추가
fix: Claude CLI 빈 에러 메시지 표시 문제 수정
docs: Architecture.md 작성
```

## 프론트엔드 개발

### 기술 스택

- **React 19** + TypeScript
- **Tailwind CSS 4** (다크 테마 기본)
- **Vite 6** (HMR)

### 컴포넌트 작성 규칙

1. **함수 컴포넌트** + hooks 패턴 사용
2. **Tailwind 클래스** 직접 사용 (별도 CSS 파일 지양)
3. **타입 안전** — props는 반드시 interface 정의
4. **Tauri IPC** — `src/lib/tauri-commands.ts`에 래퍼 함수 정의 후 사용

### 새 Tauri 커맨드 추가 시

1. `src-tauri/src/commands/`에 Rust 함수 작성 (`#[tauri::command]`)
2. `src-tauri/src/lib.rs`의 `invoke_handler`에 등록
3. `src/lib/tauri-commands.ts`에 TypeScript 래퍼 추가
4. 컴포넌트에서 래퍼 함수 import하여 사용

```typescript
// src/lib/tauri-commands.ts
export const myNewCommand = (arg: string) =>
  invoke<ResultType>("my_new_command", { arg });
```

### 다크 테마 컬러 가이드

| 용도 | 클래스 |
|------|--------|
| 배경 | `bg-gray-900` |
| 카드/패널 | `bg-gray-800` |
| 입력 필드 | `bg-gray-700` |
| 기본 텍스트 | `text-white` |
| 보조 텍스트 | `text-gray-400` |
| 비활성 텍스트 | `text-gray-500` |
| 주요 버튼 | `bg-blue-600 hover:bg-blue-500` |
| 위험 버튼 | `bg-red-600 hover:bg-red-500` |
| 성공 표시 | `text-green-400` |
| 경고 표시 | `text-yellow-500` |

## 백엔드 개발

### Rust 코드 규칙

1. **에러 처리**: `Result<T, String>` 반환 (Tauri IPC 호환)
2. **비동기**: Tauri 커맨드는 `async` 사용 가능
3. **상태 공유**: `State<'_, Mutex<AppState>>` 패턴
4. **직렬화**: `serde::Serialize`/`Deserialize` derive

### 데이터베이스 변경

- 스키마는 `src-tauri/src/session/storage.rs`의 `Database::new()`에서 관리
- 테이블 추가/변경 시 `CREATE TABLE IF NOT EXISTS` 사용
- FTS5 인덱스는 트리거로 동기화

### 새 외부 API 연동 시

1. `src-tauri/src/export/` 또는 적절한 모듈에 클라이언트 작성
2. `reqwest`로 HTTP 호출
3. 인증 정보는 `AppSettings`에 추가
4. 프론트엔드 Settings에 설정 UI 추가
5. Tauri 커맨드로 프론트엔드에 노출

## 빌드 및 릴리즈

### 로컬 빌드

```bash
# 개발 모드 (핫 리로드)
npm run tauri dev

# 프로덕션 빌드
npm run tauri build

# 결과물
# src-tauri/target/release/bundle/macos/Meering Hero.app
# src-tauri/target/release/bundle/dmg/Meering Hero_x.x.x_aarch64.dmg
```

### 아이콘 변경

```bash
# app-icon.svg 수정 후
qlmanage -t -s 1024 -o /tmp app-icon.svg
cp /tmp/app-icon.svg.png app-icon.png
npx tauri icon app-icon.png
```

### 릴리즈 프로세스

1. `tauri.conf.json`, `Cargo.toml`, `package.json`의 버전 업데이트
2. 변경사항 커밋
3. 태그 생성 및 푸시:
   ```bash
   git tag v0.2.0
   git push origin v0.2.0
   ```
4. GitHub Actions가 자동으로:
   - macOS (Apple Silicon) 빌드
   - 서명 및 업데이터 아티팩트 생성
   - Draft Release 생성
5. GitHub에서 Release 내용 작성 후 **Publish**

### 업데이터 서명 키

```bash
# 키 생성 (최초 1회)
npx tauri signer generate -w ~/.tauri/meering-hero.key

# 공개키: tauri.conf.json의 plugins.updater.pubkey에 설정
# 비밀키: GitHub Repository Secrets에 TAURI_SIGNING_PRIVATE_KEY로 설정
```

### GitHub Secrets 설정

| Secret | 설명 |
|--------|------|
| `TAURI_SIGNING_PRIVATE_KEY` | 업데이터 서명 비밀키 |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | 비밀키 비밀번호 |

Apple 코드 서명이 필요한 경우 추가:

| Secret | 설명 |
|--------|------|
| `APPLE_CERTIFICATE` | Base64 인코딩된 .p12 인증서 |
| `APPLE_CERTIFICATE_PASSWORD` | 인증서 비밀번호 |
| `APPLE_SIGNING_IDENTITY` | e.g., `Developer ID Application: Name (TEAMID)` |
| `APPLE_TEAM_ID` | Apple 팀 ID |
| `APPLE_ID` | Apple 개발자 이메일 |
| `APPLE_PASSWORD` | 앱 전용 비밀번호 |

## 테스트

### 수동 테스트 체크리스트

**녹음**:
- [ ] 새 세션 생성 → 녹음 시작 → 실시간 전사 표시
- [ ] 일시정지/재개
- [ ] 녹음 중지 → 세그먼트 저장 확인
- [ ] 다른 세션 선택 중에도 녹음 지속

**요약**:
- [ ] Claude로 요약 버튼 → 결과 표시
- [ ] 요약 편집 및 저장
- [ ] 커스텀 프롬프트 적용 확인

**내보내기**:
- [ ] 위키 목록 조회 (검색 필터링)
- [ ] 위키 페이지 트리 탐색 (depth 이동)
- [ ] 새 페이지 생성
- [ ] 기존 페이지 업데이트 (기존 제목 유지)

**설정**:
- [ ] 모델 다운로드 (진행률 표시)
- [ ] STT 모델 선택 변경
- [ ] 오디오 디바이스 변경
- [ ] Dooray 설정 저장/로드
- [ ] 업데이트 확인

## 트러블슈팅

### 빌드 오류

**whisper-rs 컴파일 에러**: Xcode Command Line Tools가 설치되어 있는지 확인
```bash
xcode-select --install
```

**포트 5173 사용 중**:
```bash
lsof -ti:5173 | xargs kill -9
```

**DMG 번들링 실패**:
```bash
brew install create-dmg
```

### 런타임 이슈

**Claude CLI 빈 에러**: stderr가 비어있으면 stdout과 exit code를 확인 (이미 처리됨)

**Dooray API 404**: API 베이스 URL이 `https://api.dooray.com`으로 자동 변환됨. 위키 API는 `/wiki/v1/` 경로 사용.

**모델 다운로드 실패**: `~/Library/Application Support/meeting-app/models/`에서 `.downloading` 파일 삭제 후 재시도
