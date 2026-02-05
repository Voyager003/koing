# Koing (코잉)

macOS에서 영문 입력 상태로 한글을 잘못 입력했을 때, 자동으로 한글로 변환해주는 유틸리티입니다.

```
rkskek → 가나다
dkssud → 안녕
gksrmf → 한글
```

## 설치

```bash
# 빌드
cargo build --release

# 실행
cargo run --release
```

## 권한 설정

**손쉬운 사용(Accessibility)** 권한이 필요합니다.

시스템 설정 → 개인 정보 보호 및 보안 → 손쉬운 사용 → Koing 허용

## 사용법

| 단축키 | 기능 |
|--------|------|
| `Option + Space` | 수동 변환 |
| `Option + Z` | 되돌리기 (Undo) |

### 자동 변환

- 영문 모드에서 한글 패턴 입력 시 자동 감지
- 300ms 동안 추가 입력이 없으면 변환
- 숫자, 특수문자 입력 시 즉시 변환

### 변환 방지

- 흔한 영어 단어 (name, code, file 등)
- 한글 입력 모드에서는 동작 안 함

## 라이선스

MIT
