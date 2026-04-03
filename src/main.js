import { Terminal } from "xterm";
import { FitAddon } from "xterm-addon-fit";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

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
    statusEl.textContent = "连接终端中...";
    await invoke("init_pty");
    statusEl.textContent = "已连接";

    await listen("pty-data", (event) => {
      term.write(event.payload);
    });

    term.onData((data) => {
      invoke("write_to_pty", { input: data });
    });
  } catch (e) {
    statusEl.textContent = "错误: " + e;
    term.write("\r\n\x1b[31m初始化失败: " + e + "\x1b[0m\r\n");
  }
}

window.sendInput = async function(text) {
  try {
    await invoke("write_to_pty", { input: text });
  } catch (e) {
    console.error(e);
  }
};

init();
