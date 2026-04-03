# Tauri 多平台构建指南

本文档指导 AI 为 Tauri 应用创建 GitHub Actions 工作流，实现：**每次推送触发构建，构建产物上传到 Artifact**。

> 参考项目：[cc-switch](https://github.com/zhaoqixiang/cc-switch) 的 `release.yml` 工作流设计。

---

## 目标

- **触发条件**：每次 `push` 到 `main` 分支（或 PR 合并时）
- **构建平台**：Windows (x64)、macOS (universal)、Linux (x64 & ARM64)
- **产物处理**：上传到 GitHub Actions Artifact，**不创建 Release**
- **Artifact 命名**：`{{app-name}}-{{os}}-{{arch}}.zip`

---

## 前置条件

确认仓库具备以下内容：

| 项目 | 说明 |
|------|------|
| `src-tauri/` | Tauri Rust 后端源码 |
| `package.json` | 前端依赖（pnpm/npm） |
| `pnpm-lock.yaml` | pnpm 锁定文件（如使用 pnpm） |
| `src-tauri/Cargo.lock` | Rust 锁定文件 |

---

## 工作流文件

新建文件：`.github/workflows/build.yml`

### 完整模板

```yaml
name: Build

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

concurrency:
  group: build-${{ github.ref }}
  cancel-in-progress: true

permissions:
  contents: read

jobs:
  build:
    name: Build ${{ matrix.os }} ${{ matrix.arch || '' }}
    runs-on: ${{ matrix.runs-on }}
    strategy:
      fail-fast: false
      matrix:
        include:
          - os: windows-latest
            runs-on: windows-2022
            arch: x64
            bundle-suffix: Windows-x64
          - os: ubuntu-latest
            runs-on: ubuntu-22.04
            arch: x64
            bundle-suffix: Linux-x86_64
          - os: ubuntu-latest-arm
            runs-on: ubuntu-22.04-arm
            arch: arm64
            bundle-suffix: Linux-arm64
          - os: macos-latest
            runs-on: macos-14
            arch: arm64
            bundle-suffix: macOS-arm64
          - os: macos-latest-x64
            runs-on: macos-14
            arch: x64
            bundle-suffix: macOS-x86_64

    steps:
      - name: Checkout
        uses: actions/checkout@v4

      # ── Node.js ────────────────────────────────────────
      - name: Setup Node.js
        uses: actions/setup-node@v6
        with:
          node-version: '20'

      # ── pnpm ──────────────────────────────────────────
      - name: Setup pnpm
        uses: pnpm/action-setup@v5
        with:
          version: 10.12.3
          run_install: false

      - name: Get pnpm store directory
        id: pnpm-store
        shell: bash
        run: echo "path=$(pnpm store path --silent)" >> $GITHUB_OUTPUT

      - name: Cache pnpm store
        uses: actions/cache@v5
        with:
          path: ${{ steps.pnpm-store.outputs.path }}
          key: ${{ runner.os }}-${{ runner.arch }}-pnpm-store-${{ hashFiles('**/pnpm-lock.yaml') }}
          restore-keys: ${{ runner.os }}-${{ runner.arch }}-pnpm-store-

      - name: Install dependencies
        run: pnpm install --frozen-lockfile

      # ── Rust ──────────────────────────────────────────
      - name: Setup Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Add macOS targets
        if: runner.os == 'macOS'
        run: |
          rustup target add aarch64-apple-darwin x86_64-apple-darwin

      - name: Cache Cargo registry and build
        uses: actions/cache@v5
        with:
          path: |
            ~/.cargo/registry
            ~/.cargo/git
            src-tauri/target
          key: ${{ runner.os }}-cargo-${{ hashFiles('src-tauri/Cargo.lock') }}
          restore-keys: ${{ runner.os }}-cargo-

      # ── Linux 系统依赖 ─────────────────────────────────
      - name: Install Linux system deps
        if: runner.os == 'Linux'
        shell: bash
        run: |
          set -euxo pipefail
          sudo apt-get update
          sudo apt-get install -y --no-install-recommends \
            build-essential \
            pkg-config \
            libssl-dev \
            curl \
            wget \
            file \
            patchelf \
            libgtk-3-dev \
            librsvg2-dev \
            libayatana-appindicator3-dev
          sudo apt-get install -y --no-install-recommends libwebkit2gtk-4.1-dev \
            || sudo apt-get install -y --no-install-recommends libwebkit2gtk-4.0-dev
          sudo apt-get install -y --no-install-recommends libsoup-3.0-dev \
            || sudo apt-get install -y --no-install-recommends libsoup2.4-dev

      # ── Tauri 构建 ─────────────────────────────────────
      - name: Build Tauri App (Windows)
        if: runner.os == 'Windows'
        shell: pwsh
        run: pnpm tauri build

      - name: Build Tauri App (macOS)
        if: runner.os == 'macOS'
        shell: bash
        timeout-minutes: 60
        run: |
          if [ "${{ matrix.arch }}" == "x64" ]; then
            pnpm tauri build --target x86_64-apple-darwin
          elif [ "${{ matrix.arch }}" == "arm64" ]; then
            pnpm tauri build --target aarch64-apple-darwin
          else
            pnpm tauri build --target universal-apple-darwin
          fi

      - name: Build Tauri App (Linux)
        if: runner.os == 'Linux'
        shell: bash
        run: pnpm tauri build --bundles appimage,deb,rpm

      # ── 收集产物 ─────────────────────────────────────
      - name: Collect Windows Assets
        if: runner.os == 'Windows'
        shell: pwsh
        run: |
          $ErrorActionPreference = 'Stop'
          New-Item -ItemType Directory -Force -Path release-assets | Out-Null

          # MSI
          $msi = Get-ChildItem -Path 'src-tauri/target/release/bundle/msi' -Recurse -Include *.msi -ErrorAction SilentlyContinue | Select-Object -First 1
          if ($null -ne $msi) {
            Copy-Item $msi.FullName "release-assets/$($msi.Name)" -Force
            $sigPath = "$($msi.FullName).sig"
            if (Test-Path $sigPath) { Copy-Item $sigPath "release-assets/$($msi.Name).sig" -Force }
          }

          # 便携版 exe
          $exeCandidates = @(
            'src-tauri/target/release/*.exe',
            'src-tauri/target/x86_64-pc-windows-msvc/release/*.exe'
          )
          $exe = Get-ChildItem $exeCandidates -ErrorAction SilentlyContinue | Select-Object -First 1
          if ($null -ne $exe) {
            $portableDir = 'release-assets/portable'
            New-Item -ItemType Directory -Force -Path $portableDir | Out-Null
            Copy-Item $exe.FullName "$portableDir/"
            Compress-Archive -Path "$portableDir/*" -DestinationPath "release-assets/portable.zip" -Force
            Remove-Item -Recurse -Force $portableDir
          }

          Get-ChildItem release-assets | ForEach-Object { Write-Host "  $_" }

      - name: Collect macOS Assets
        if: runner.os == 'macOS'
        shell: bash
        run: |
          set -euxo pipefail
          mkdir -p release-assets

          # 查找 .app
          APP_PATH=""
          for path in \
            "src-tauri/target/universal-apple-darwin/release/bundle/macos" \
            "src-tauri/target/aarch64-apple-darwin/release/bundle/macos" \
            "src-tauri/target/x86_64-apple-darwin/release/bundle/macos" \
            "src-tauri/target/release/bundle/macos"; do
            if [ -d "$path" ]; then
              [ -z "$APP_PATH" ] && APP_PATH=$(find "$path" -maxdepth 1 -name "*.app" -type d | head -1 || true)
            fi
          done

          if [ -n "$APP_PATH" ]; then
            ditto -c -k --sequesterRsrc --keepParent "$APP_PATH" "release-assets/app.zip"
            echo "macOS app.zip created"
          fi

          # .tar.gz (updater)
          TAR_GZ=$(find src-tauri/target -name "*.tar.gz" -type f | head -1 || true)
          if [ -n "$TAR_GZ" ]; then
            cp "$TAR_GZ" "release-assets/app.tar.gz"
            [ -f "$TAR_GZ.sig" ] && cp "$TAR_GZ.sig" "release-assets/app.tar.gz.sig"
            echo "macOS tarball copied"
          fi

          ls -la release-assets/

      - name: Collect Linux Assets
        if: runner.os == 'Linux'
        shell: bash
        run: |
          set -euxo pipefail
          mkdir -p release-assets

          # AppImage
          APPIMAGE=$(find src-tauri/target/release/bundle -name "*.AppImage" | head -1 || true)
          if [ -n "$APPIMAGE" ]; then
            cp "$APPIMAGE" "release-assets/app.AppImage"
            [ -f "$APPIMAGE.sig" ] && cp "$APPIMAGE.sig" "release-assets/app.AppImage.sig"
          fi

          # deb
          DEB=$(find src-tauri/target/release/bundle -name "*.deb" | head -1 || true)
          [ -n "$DEB" ] && cp "$DEB" "release-assets/app.deb"

          # rpm
          RPM=$(find src-tauri/target/release/bundle -name "*.rpm" | head -1 || true)
          [ -n "$RPM" ] && cp "$RPM" "release-assets/app.rpm"

          ls -la release-assets/

      # ── 上传 Artifact ─────────────────────────────────
      - name: Upload Artifact
        uses: actions/upload-artifact@v7
        with:
          name: ${{ github.event.repository.name }}-${{ matrix.bundle-suffix }}
          path: release-assets/*
          if-no-files-found: error

      # ── 调试：列出所有产物 ────────────────────────────
      - name: List generated bundles
        if: always()
        shell: bash
        run: |
          echo "=== Bundles in src-tauri/target ==="
          find src-tauri/target -maxdepth 5 -type f \( -name "*.exe" -o -name "*.msi" -o -name "*.app" -o -name "*.AppImage" -o -name "*.deb" -o -name "*.rpm" -o -name "*.tar.gz" \) 2>/dev/null | head -20 || true
```

---

## 关键差异说明（对比 cc-switch release.yml）

| 项目 | release.yml (cc-switch) | build.yml (本项目) |
|------|--------------------------|---------------------|
| 触发条件 | `push` tag (`v*`) | `push`/`pull_request` 分支 |
| 权限 | `contents: write` | `contents: read` |
| 构建产物 | 上传到 GitHub Release | 上传到 Artifact |
| 发布步骤 | `softprops/action-gh-release` | 无 |
| latest.json | 需要生成 | 不需要 |
| Apple 签名/公证 | 需要 Secrets | 不需要 |
| Tauri signing key | 需要 Secrets | 不需要 |

---

## Artifact 下载说明

构建完成后，进入 **Actions** → 对应 Run → **Artifacts** 区域下载。

命名规则：`{仓库名}-{平台}.zip`，例如：
- `ti-Windows-x64.zip`
- `ti-Linux-x86_64.tar.gz`
- `ti-macOS-arm64.zip`

---

## 平台产物清单

| 平台 | 主要产物 | 说明 |
|------|----------|------|
| Windows | `.msi`、`.exe` (portable zip) | MSI 用于安装，portable 绿色运行 |
| macOS | `.app.zip`、`.tar.gz` | zip 为分发用，tar.gz 为 Tauri updater |
| Linux x64 | `.AppImage`、`.deb`、`.rpm` | AppImage 通用可执行 |

---

## 注意事项

1. **macOS universal build**：若没有 macOS ARM64 runner，可简化为只构建 `universal-apple-darwin` 单产物
2. **Linux ARM64**：需要 `ubuntu-22.04-arm` runner（GitHub 托管的 ARM runner）
3. **Node 版本**：建议与本地开发环境保持一致，避免 pnpm lock 不兼容
4. **pnpm lock**：务必提交 `pnpm-lock.yaml`，否则 CI 会失败
5. **超时**：macOS 构建建议设置 `timeout-minutes: 60`，防止网络波动导致超时

---

## 常见问题

**Q: macOS 构建失败，提示 `Couldn't find rustup toolchain`？**
> 确保 `rust-toolchain@stable` 在 `Setup Node.js` 之前执行，且检查是否需要手动添加 target。

**Q: Linux 构建提示 `libwebkit2gtk` 找不到？**
> 参考 `apt-get install` 步骤，同时安装 `libwebkit2gtk-4.1-dev` 和 `libwebkit2gtk-4.0-dev` 作为兜底。

**Q: 如何增加新平台？**
> 在 matrix `include` 中新增一项，填写 `os`、`runs-on`、`arch`、`bundle-suffix`，并在 Build 和 Collect 步骤添加对应的 `if: runner.os == '...'` 条件块。
