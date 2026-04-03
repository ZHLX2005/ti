# ti - 终端验证工具

基于 Tauri 2.0 + portable-pty + xterm.js 的终端验证MVP。

## 功能

- 虚拟按键发送（Ctrl+C, Ctrl+Z等）
- 终端输出实时显示
- Windows amd64 支持

## 开发

```bash
npm install
npm run tauri dev
```

## 构建

```bash
npm run tauri build
```

## GitHub Actions

推送到 main 分支自动构建 Windows amd64 安装包。
