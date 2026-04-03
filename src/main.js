import { Terminal } from "xterm";
import { FitAddon } from "xterm-addon-fit";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

const terminalContainer = document.getElementById("terminal");
const statusEl = document.getElementById("status");

let term;
let fitAddon;

async function init() {
  term = new Terminal({ cursorBlink: true, fontSize: 14 });
  fitAddon = new FitAddon();
  term.loadAddon(fitAddon);
  term.open(terminalContainer);
  fitAddon.fit();

  window.addEventListener("resize", () => fitAddon.fit());

  try {
    statusEl.textContent = "就绪";
    term.write("终端验证工具\r\n");

    await listen("command-output", (event) => {
      term.write(event.payload);
    });

    term.onData((data) => {
      if (data === "\r") {
        term.write("\r\n");
      } else {
        term.write(data);
      }
    });
  } catch (e) {
    statusEl.textContent = "错误: " + e;
    term.write("\r\n\x1b[31m初始化失败\x1b[0m\r\n");
  }
}

window.sendCommand = async function(cmd) {
  try {
    await invoke("run_command", { cmd });
  } catch (e) {
    term.write("\r\n\x1b[31m错误: " + e + "\x1b[0m\r\n");
  }
};

init();
