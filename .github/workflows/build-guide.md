# Tauri 多平台构建指南

参考 [cc-switch](https://github.com/zhaoqixiang/cc-switch) 的 `release.yml`，每次 push 到 main 触发构建，构建产物上传 Artifact。

---

## 工作流文件

`.github/workflows/build.yml` 包含全部 5 个平台的构建矩阵：

| 平台 | Runner | 产物 |
|------|--------|------|
| Windows x64 | `windows-2022` | `.msi`、portable zip |
| Linux x64 | `ubuntu-22.04` | `.deb`、`.rpm` |
| Linux ARM64 | `ubuntu-22.04-arm` | `.deb`、`.rpm` |
| macOS ARM64 | `macos-14` | `.app.zip`、`.tar.gz` |
| macOS x64 | `macos-14` | `.app.zip`、`.tar.gz` |

---

## 触发与权限

```yaml
on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

permissions:
  contents: read   # 只读，无需写入 contents
```

---

## 平台差异对照

| 步骤 | Windows | Linux | macOS |
|------|---------|-------|-------|
| 系统依赖 | 无 | `apt-get install` GTK/WebKit | rustup target |
| 构建命令 | `pnpm tauri build` | `--bundles deb,rpm` | `--target x86_64-apple-darwin` 等 |
| 产物收集 | MSI + exe zip | deb + rpm | app.zip + tar.gz |

---

## Artifact 命名

上传到 Actions Artifact，命名为 `{仓库名}-{平台}`，例如：
- `ti-Windows-x64`
- `ti-Linux-x86_64`
- `ti-macOS-arm64`

---

## 注意事项

1. **Linux AppImage**：需要 512x512 正方形图标 `src-tauri/icons/icon.png`，当前缺失故未包含
2. **macOS 签名公证**：本 workflow 不包含（需要 Secrets），仅构建产物
3. **pnpm lock**：必须提交 `pnpm-lock.yaml`，否则 `--frozen-lockfile` 会失败
4. **Cargo.lock**：必须提交 `src-tauri/Cargo.lock`，否则缓存 key 不生效
