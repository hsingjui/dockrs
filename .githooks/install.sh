#!/usr/bin/env sh
# 安装 git hooks：指向仓库内 .githooks 目录（hooksPath 无法提交进 git，需本地设置一次）
dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
git config core.hooksPath "$dir"
echo "Git hooks 已安装: $(git config --get core.hooksPath)"
