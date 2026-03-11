#!/usr/bin/env bash
set -euo pipefail

if [[ -z "${HOMEBREW_TAP_TOKEN:-}" ]]; then
  echo "HOMEBREW_TAP_TOKEN is required"
  exit 1
fi

if [[ -z "${VERSION:-}" || -z "${SHA256:-}" ]]; then
  echo "VERSION and SHA256 are required"
  exit 1
fi

TAP_REPO="${TAP_REPO:-Voyager003/homebrew-koing}"
WORKDIR="$(mktemp -d)"
cleanup() {
  rm -rf "$WORKDIR"
}
trap cleanup EXIT

git clone "https://x-access-token:${HOMEBREW_TAP_TOKEN}@github.com/${TAP_REPO}.git" "$WORKDIR/tap"

cat >"$WORKDIR/tap/Casks/koing.rb" <<EOF
cask "koing" do
  version "${VERSION}"
  sha256 "${SHA256}"

  url "https://github.com/Voyager003/koing/releases/download/v#{version}/Koing-#{version}.zip"
  name "Koing"
  desc "macOS Korean-English auto-converter"
  homepage "https://github.com/Voyager003/koing"

  depends_on macos: ">= :ventura"
  depends_on arch: :arm64

  app "Koing.app"

  postflight do
    system_command "/usr/bin/xattr",
                   args: ["-d", "com.apple.quarantine", "#{appdir}/Koing.app"]
    # 재설치/업그레이드 시 이전 빌드의 stale TCC 항목 제거
    system_command "/usr/bin/tccutil",
                   args: ["reset", "Accessibility", "com.koing.app"]
  end
end
EOF

git -C "$WORKDIR/tap" config user.name "github-actions[bot]"
git -C "$WORKDIR/tap" config user.email "41898282+github-actions[bot]@users.noreply.github.com"

if git -C "$WORKDIR/tap" diff --quiet -- Casks/koing.rb; then
  echo "Homebrew tap already up to date"
  exit 0
fi

git -C "$WORKDIR/tap" add Casks/koing.rb
git -C "$WORKDIR/tap" commit -m "koing ${VERSION}"
git -C "$WORKDIR/tap" push origin HEAD:main
