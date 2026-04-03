import { Terminal } from "xterm";
import { FitAddon } from "xterm-addon-fit";
import { getMatches } from "@tauri-apps/plugin-cli";
import { invoke } from "@tauri-apps/api/core";

const terminalContainer = document.getElementById("terminal");
const statusEl = document.getElementById("status");

let term;

async function init() {
  term = new Terminal({ cursorBlink: true, fontSize: 14 });
  const fitAddon = new FitAddon();
  term.loadAddon(fitAddon);
  term.open(terminalContainer);
  fitAddon.fit();

  window.addEventListener("resize", () => fitAddon.fit());

  try {
    const matches = await getMatches();
    const args = matches.args;

    if (args.command?.value) {
      statusEl.textContent = "执行命令: " + args.command.value;
      const cmd = args.command.value;
      const result = await invoke("run_command", { cmd });
      term.writeln(result);
    } else {
      statusEl.textContent = "终端已就绪";
      term.writeln("Tauri CLI 终端验证工具");
      term.writeln("用法: ti -c <命令>");
      term.writeln("");
    }

    if (args.verbose) {
      term.writeln("Verbose 模式已启用");
    }
  } catch (e) {
    statusEl.textContent = "错误: " + e;
    term.writeln("\r\n\x1b[31m初始化失败: " + e + "\x1b[0m");
  }
}

init();
