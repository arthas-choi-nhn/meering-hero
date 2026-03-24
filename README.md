# Meering Hero

사내 회의록 자동화 데스크톱 앱. 음성을 로컬에서 실시간 전사(STT)하고, Claude로 요약한 뒤, Dooray Wiki에 내보냅니다.

## 주요 기능

- **로컬 STT** — whisper.cpp + CoreML로 MacBook에서 실시간 음성 인식 (외부 서버 전송 없음)
- **AI 요약** — Claude Code CLI를 활용한 회의록 자동 생성
- **Dooray Wiki 연동** — 전사본 + 요약 + 노트를 위키 페이지로 내보내기
- **오프라인 저장** — SQLite 기반 세션/전사/요약 로컬 저장 및 검색

## 시스템 요구사항

| 항목 | 요구사항 |
|------|---------|
| OS | macOS 13.0+ (Ventura 이상) |
| 칩 | Apple Silicon (M1/M2/M3/M4) |
| RAM | 8GB 이상 (16GB 추천) |
| Claude Code | 설치 및 로그인 완료 (요약 기능용) |

## 설치

### DMG로 설치

1. [Releases](https://github.com/nhn-cloud/meering-hero/releases)에서 최신 `.dmg` 파일 다운로드
2. DMG를 열고 `Meering Hero.app`을 Applications 폴더로 드래그
3. 첫 실행 시 "확인되지 않은 개발자" 경고가 뜨면:
   ```bash
   xattr -cr "/Applications/Meering Hero.app"
   ```

### 자동 업데이트

앱 설치 후 **설정 > 앱 업데이트 > 업데이트 확인**을 클릭하면 최신 버전이 있을 때 자동으로 다운로드 및 설치됩니다.

## 초기 설정

### 1. STT 모델 다운로드

앱 실행 후 **설정**(우측 상단 톱니바퀴) > **모델 관리**에서:

1. **Silero VAD** 다운로드 (~2MB) — 음성 구간 감지용
2. **Whisper 모델** 다운로드 — RAM에 따라 추천 모델이 표시됨

| 모델 | RAM | 크기 | 정확도 |
|------|-----|------|--------|
| Small | 8GB+ | ~500MB | 기본 |
| Medium | 16GB+ | ~1.5GB | 추천 |
| Large | 32GB+ | ~3GB | 최고 |

### 2. Claude Code 설정

요약 기능을 사용하려면 [Claude Code](https://claude.ai/claude-code)가 설치되어 있어야 합니다.

```bash
# Claude Code 설치 (아직 없다면)
npm install -g @anthropic-ai/claude-code

# 로그인
claude login
```

설정 화면에서 **Claude Code CLI** 섹션이 "사용 가능"으로 표시되는지 확인하세요.

### 3. Dooray Wiki 연동 (선택)

설정 > **Dooray Wiki API**에서:
- **Base URL**: `https://your-org.dooray.com`
- **API Token**: Dooray 설정에서 발급한 API 토큰

## 사용법

### 회의 녹음

1. **+ 새 회의** 클릭 → 제목, 참석자, 맥락 힌트(전문용어) 입력
2. **녹음 시작** 버튼 클릭 → 실시간 전사 시작
3. 회의 종료 시 **녹음 중지** 클릭

### 요약 생성

1. 녹음 종료 후 우측 패널의 **Claude로 요약** 클릭
2. 요약 결과 확인 후 필요 시 **편집** 가능
3. 커스텀 프롬프트는 설정 > **요약 프롬프트**에서 수정

### Dooray Wiki 내보내기

1. 우측 패널 하단 **Dooray Wiki로 내보내기** 클릭
2. **새 위키 페이지** 또는 **기존 페이지에 작성** 선택
3. 위키 선택 → 위치 선택 → 생성/업데이트

내보내기 내용: 회의 정보 + 요약 + 노트 + 전사본(접기)

## 개발 환경 설정

### 필수 도구

- Node.js 18+
- Rust (stable)
- Xcode Command Line Tools

### 실행

```bash
# 의존성 설치
npm install

# 개발 모드 실행
npm run tauri dev

# 프로덕션 빌드
npm run tauri build
```

### 기술 스택

| 레이어 | 기술 |
|--------|------|
| 프레임워크 | Tauri 2 |
| 프론트엔드 | React 19 + TypeScript + Tailwind CSS 4 |
| 백엔드 | Rust |
| STT | whisper.cpp (CoreML) |
| VAD | Silero VAD (ONNX Runtime) |
| 요약 | Claude Code CLI |
| DB | SQLite + FTS5 |
| 빌드 | Vite 6 |

## 라이선스

Internal use only - NHN Cloud
