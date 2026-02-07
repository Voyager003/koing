#!/usr/bin/env python3
"""
N-gram 학습 스크립트

한국어 텍스트 코퍼스에서 유니그램/바이그램 빈도를 수집하여
ngram_model.json 파일을 생성합니다.

사용법:
    python train.py ./corpus.txt -o ./ngram_model.json --min-freq 5
    python train.py --generate-sample -o ./ngram_model.json
"""

import argparse
import json
import re
import sys
from collections import Counter
from pathlib import Path


# 한글 완성형 유니코드 범위
HANGUL_START = 0xAC00  # '가'
HANGUL_END = 0xD7A3    # '힣'


def is_hangul_syllable(char: str) -> bool:
    """완성형 한글인지 확인"""
    if len(char) != 1:
        return False
    code = ord(char)
    return HANGUL_START <= code <= HANGUL_END


def extract_hangul(text: str) -> str:
    """텍스트에서 완성형 한글만 추출"""
    return ''.join(c for c in text if is_hangul_syllable(c))


def count_ngrams(text: str, min_freq: int = 1) -> tuple[Counter, Counter]:
    """
    텍스트에서 유니그램과 바이그램 빈도를 계산

    Args:
        text: 한글 텍스트
        min_freq: 최소 빈도수 (이하는 제외)

    Returns:
        (unigrams Counter, bigrams Counter)
    """
    hangul_only = extract_hangul(text)

    # 유니그램 카운트
    unigrams = Counter(hangul_only)

    # 바이그램 카운트
    bigrams = Counter()
    for i in range(len(hangul_only) - 1):
        bigram = hangul_only[i:i+2]
        bigrams[bigram] += 1

    # 최소 빈도 필터링
    if min_freq > 1:
        unigrams = Counter({k: v for k, v in unigrams.items() if v >= min_freq})
        bigrams = Counter({k: v for k, v in bigrams.items() if v >= min_freq})

    return unigrams, bigrams


def generate_sample_model() -> dict:
    """
    테스트용 샘플 N-gram 모델 생성

    일반적인 한국어 패턴을 기반으로 한 샘플 데이터
    """
    # 샘플 코퍼스: 일반적인 한국어 문장들
    sample_corpus = """
    안녕하세요 반갑습니다 감사합니다 죄송합니다
    오늘 날씨가 좋습니다 내일 비가 올 것 같습니다
    한글은 세종대왕이 창제하였습니다
    프로그래밍을 공부하고 있습니다
    코딩은 재미있습니다 개발자가 되고 싶습니다
    이것은 테스트입니다 잘 동작하나요
    컴퓨터 과학을 전공하고 있습니다
    맥북에서 한영 변환을 자동으로 해줍니다
    두벌식 자판으로 한글을 입력합니다
    가나다라마바사아자차카타파하
    아버지가 방에 들어가신다
    대한민국 만세
    사랑합니다 행복하세요
    좋은 하루 되세요
    수고하셨습니다
    """

    # 추가 빈도 데이터 (일반적인 한글 패턴)
    common_patterns = [
        "니다", "습니", "하세", "세요", "합니", "니까",
        "안녕", "감사", "죄송", "반갑", "축하", "미안",
        "으로", "에서", "하고", "이고", "그리", "하지",
        "입니", "했습", "하였", "되었", "있습", "없습",
        "오늘", "내일", "어제", "지금", "나중", "먼저",
        "한글", "영문", "변환", "입력", "출력", "처리",
    ]

    # 코퍼스에서 기본 카운트
    unigrams, bigrams = count_ngrams(sample_corpus * 10, min_freq=1)

    # 일반적인 패턴에 가중치 부여
    for pattern in common_patterns:
        hangul = extract_hangul(pattern)
        for char in hangul:
            unigrams[char] += 100
        for i in range(len(hangul) - 1):
            bigrams[hangul[i:i+2]] += 50

    # 자주 사용되는 음절에 추가 가중치
    common_syllables = "가나다라마바사아자차카타파하이은는을를에서로의가"
    for char in common_syllables:
        unigrams[char] += 200

    corpus_size = sum(unigrams.values())

    return {
        "metadata": {
            "corpus_size": corpus_size,
            "unique_unigrams": len(unigrams),
            "unique_bigrams": len(bigrams),
            "source": "sample_generated",
        },
        "unigrams": dict(unigrams.most_common()),
        "bigrams": {f"{k[0]}|{k[1]}": v for k, v in bigrams.most_common()},
    }


def train_from_corpus(corpus_path: Path, min_freq: int = 5) -> dict:
    """
    코퍼스 파일에서 N-gram 모델 학습

    Args:
        corpus_path: 코퍼스 파일 경로
        min_freq: 최소 빈도수

    Returns:
        N-gram 모델 딕셔너리
    """
    if not corpus_path.exists():
        raise FileNotFoundError(f"코퍼스 파일을 찾을 수 없습니다: {corpus_path}")

    print(f"코퍼스 파일 읽는 중: {corpus_path}")

    with open(corpus_path, 'r', encoding='utf-8') as f:
        text = f.read()

    print(f"총 문자 수: {len(text):,}")

    hangul_text = extract_hangul(text)
    print(f"한글 문자 수: {len(hangul_text):,}")

    print(f"N-gram 카운트 중 (min_freq={min_freq})...")
    unigrams, bigrams = count_ngrams(text, min_freq=min_freq)

    corpus_size = sum(unigrams.values())

    print(f"유니그램: {len(unigrams):,}개")
    print(f"바이그램: {len(bigrams):,}개")

    return {
        "metadata": {
            "corpus_size": corpus_size,
            "unique_unigrams": len(unigrams),
            "unique_bigrams": len(bigrams),
            "min_freq": min_freq,
            "source": str(corpus_path),
        },
        "unigrams": dict(unigrams.most_common()),
        "bigrams": {f"{k[0]}|{k[1]}": v for k, v in bigrams.most_common()},
    }


def save_model(model: dict, output_path: Path) -> None:
    """모델을 JSON 파일로 저장"""
    output_path.parent.mkdir(parents=True, exist_ok=True)

    with open(output_path, 'w', encoding='utf-8') as f:
        json.dump(model, f, ensure_ascii=False, indent=2)

    print(f"모델 저장 완료: {output_path}")
    print(f"파일 크기: {output_path.stat().st_size:,} bytes")


def main():
    parser = argparse.ArgumentParser(
        description="한국어 N-gram 모델 학습 스크립트",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
예시:
    # 코퍼스 파일에서 학습
    python train.py ./corpus.txt -o ./ngram_model.json --min-freq 5

    # 샘플 모델 생성 (테스트용)
    python train.py --generate-sample -o ./ngram_model.json
        """
    )

    parser.add_argument(
        "corpus",
        nargs="?",
        type=Path,
        help="한국어 텍스트 코퍼스 파일 경로"
    )

    parser.add_argument(
        "-o", "--output",
        type=Path,
        default=Path("ngram_model.json"),
        help="출력 파일 경로 (기본: ngram_model.json)"
    )

    parser.add_argument(
        "--min-freq",
        type=int,
        default=5,
        help="최소 빈도수 (기본: 5)"
    )

    parser.add_argument(
        "--generate-sample",
        action="store_true",
        help="테스트용 샘플 모델 생성"
    )

    args = parser.parse_args()

    if args.generate_sample:
        print("샘플 N-gram 모델 생성 중...")
        model = generate_sample_model()
    elif args.corpus:
        model = train_from_corpus(args.corpus, min_freq=args.min_freq)
    else:
        parser.error("코퍼스 파일 경로 또는 --generate-sample 옵션이 필요합니다")
        return

    save_model(model, args.output)

    # 통계 출력
    print("\n=== 모델 통계 ===")
    print(f"코퍼스 크기: {model['metadata']['corpus_size']:,}")
    print(f"유니그램 수: {model['metadata']['unique_unigrams']:,}")
    print(f"바이그램 수: {model['metadata'].get('unique_bigrams', len(model['bigrams'])):,}")

    # 상위 유니그램 출력
    print("\n상위 10개 유니그램:")
    for char, count in list(model['unigrams'].items())[:10]:
        print(f"  {char}: {count:,}")

    # 상위 바이그램 출력
    print("\n상위 10개 바이그램:")
    for bigram, count in list(model['bigrams'].items())[:10]:
        chars = bigram.split('|')
        print(f"  {chars[0]}{chars[1]}: {count:,}")


if __name__ == "__main__":
    main()
