# N-gram 학습 도구

한국어 텍스트 코퍼스에서 유니그램/바이그램 빈도를 수집하여 N-gram 모델을 생성합니다.

## 요구사항

- Python 3.9+
- 추가 패키지 불필요 (표준 라이브러리만 사용)

## 사용법

### 샘플 모델 생성 (테스트용)

```bash
python train.py --generate-sample -o ../../data/ngram_model.json
```

### 코퍼스에서 학습

```bash
python train.py ./corpus.txt -o ./ngram_model.json --min-freq 5
```

## 옵션

| 옵션 | 설명 | 기본값 |
|------|------|--------|
| `corpus` | 한국어 텍스트 코퍼스 파일 경로 | - |
| `-o, --output` | 출력 파일 경로 | `ngram_model.json` |
| `--min-freq` | 최소 빈도수 (이하는 제외) | 5 |
| `--generate-sample` | 테스트용 샘플 모델 생성 | - |

## 출력 형식

```json
{
  "metadata": {
    "corpus_size": 123456,
    "unique_unigrams": 4521,
    "unique_bigrams": 28450
  },
  "unigrams": {
    "이": 234567,
    "름": 45678
  },
  "bigrams": {
    "이|름": 128450,
    "안|녕": 98765
  }
}
```

## 한글 코퍼스 수집

공개된 한국어 코퍼스:
- [모두의 말뭉치](https://corpus.korean.go.kr/)
- [AIHub 한국어 데이터셋](https://aihub.or.kr/)
- [나무위키 덤프](https://namu.wiki/)

## 모델 사용

생성된 `ngram_model.json`은 Rust 런타임에서 로드하여 한영 변환 검증에 사용됩니다.

```rust
use koing::ngram::NgramModel;

let model = NgramModel::load("data/ngram_model.json")?;
let score = model.score("안녕하세요");
```
