#!/usr/bin/env bash
# 构建 release 并把资源放到可执行文件旁边（Bevy 默认相对 exe 找 assets/）。
set -euo pipefail
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

cargo build -p ooi_app --release

out="target/release"
rm -rf "$out/assets"
cp -R assets "$out/assets"

echo "打包完成：$root/$out/ooi_app"
echo "运行：$root/$out/ooi_app"
