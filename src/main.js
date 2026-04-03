import { Terminal } from "xterm";
import { FitAddon } from "xterm-addon-fit";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

const terminalContainer = document.getElementById("terminal");
const statusEl = document.getElementById("status");

let term;
let currentLine = "";

async function init() {
  term = new Terminal({ cursorBlink: true, fontSize: 14 });
  fitAddon = new FitAddon();
  term.loadAddon(fitAddon);
  term.open(terminalContainer);
  fitAddon.fit();

  window.addEventListener("resize", () => fitAddon.fit());

  try {
    statusEl.textContent = "连接中...";
    await invoke("init_shell");
    statusEl.textContent = "已连接";

    await listen("pty-data", (event) => {
      term.write(event.payload);
    });

    term.onData((data) => {
      if (data === "\r") {
        term.write("\r\n");
        if (currentLine.trim()) {
          invoke("write_to_shell", { input: currentLine.trim() });
        }
        currentLine = "";
        term.write("$ ");
      } else if (data === "\x7f") {
        // Backspace
        if (currentLine.length > 0) {
          currentLine = currentLine.slice(0, -1);
          term.write("\b \b");
        }
      } else {
        currentLine += data;
        term.write(data);
      }
    });
  } catch (e) {
    statusEl.textContent = "错误: " + e;
    term.write("\r\n\x1b[31m初始化失败\x1b[0m\r\n");
  }
}

init();
