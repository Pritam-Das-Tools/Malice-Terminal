const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

// --- Global Variables ---
let terminalWindowEl;
let terminalHistoryEl;
let currentPrompt = "pritam@habra:~$";

// --- Helper Functions ---

// Creates a new, active input line at the bottom of the terminal
function createNewInputLine() {
  const inputLine = document.createElement("div");
  inputLine.className = "terminal-input-line";
  inputLine.id = "active-line";

  const promptEl = document.createElement("span");
  promptEl.className = "prompt";
  promptEl.textContent = currentPrompt;

  const inputEl = document.createElement("input");
  inputEl.type = "text";
  inputEl.id = "command-input";
  inputEl.autofocus = true;

  inputEl.addEventListener("keydown", (e) => {
    if (e.key === "Enter") {
      e.preventDefault();
      executeCommand(inputLine);
    }
  });
  
  inputLine.appendChild(promptEl);
  inputLine.appendChild(inputEl);
  terminalWindowEl.appendChild(inputLine);

  inputEl.focus();
  terminalWindowEl.scrollTop = terminalWindowEl.scrollHeight;
}

// --- Main Logic ---

// Executes the command from a given input element
async function executeCommand(inputLineEl) {
  const inputEl = inputLineEl.querySelector('input');
  const command = inputEl.value;

  // Make the used input line a permanent part of the history
  inputLineEl.removeAttribute('id');
  inputEl.setAttribute("readonly", true);
  inputEl.id = "";
  terminalHistoryEl.appendChild(inputLineEl);

  // --- Logic to handle commands ---
  if (command.trim().toLowerCase() === "clear") {
    terminalHistoryEl.innerHTML = '';
  } else if (command.trim() !== "") {
    // Create a container for this command's output
    const outputContainer = document.createElement('div');
    outputContainer.className = 'output-container';
    terminalHistoryEl.appendChild(outputContainer);

    try {
      await invoke("handle_command", { command });
    } catch (error) {
      const errorLine = document.createElement("pre");
      errorLine.textContent = `Error: ${error}`;
      outputContainer.appendChild(errorLine);
    }
  }

  // Create the next prompt
  createNewInputLine();
}

// --- Initial Setup ---

window.addEventListener("DOMContentLoaded", async () => {
  terminalWindowEl = document.querySelector("#terminal-window");
  terminalHistoryEl = document.querySelector("#terminal-history");

  // This single, global listener will handle all output from Rust.
  await listen('terminal-output', (event) => {
    // Find the last output container created and append text to it.
    const lastOutputContainer = terminalHistoryEl.querySelector('.output-container:last-of-type');
    if (lastOutputContainer) {
      const outputLine = document.createElement("pre");
      outputLine.textContent = event.payload;
      lastOutputContainer.appendChild(outputLine);
      terminalWindowEl.scrollTop = terminalWindowEl.scrollHeight;
    }
  });
  
  // This listener handles path updates for the prompt.
  await listen('path-update', (event) => {
    currentPrompt = `pritam@habra:${event.payload}$`;
    const activePrompt = document.querySelector("#active-line .prompt");
    if (activePrompt) {
      activePrompt.textContent = currentPrompt;
    }
  });

  createNewInputLine();
});