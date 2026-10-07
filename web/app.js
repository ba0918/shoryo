// The top-level component: holds the server's view and the screen-wide state (the tab and
// the open dialog), turns them into each region's view data, and carries the person's
// actions to the server. No region talks to another; everything passes through here.
import { h } from "./dom.js";
import { CurrentRound } from "./components/current-round.js";
import { DialogLayer } from "./components/dialog.js";
import { Header, Tabs } from "./components/header.js";
import { currentRoundData, decisionDetail } from "./view-data.js";

const TABS = [
  { id: "current", name: "Current round" },
  { id: "past", name: "Past rounds" },
  { id: "map", name: "Map" },
  { id: "decisions", name: "Decisions" },
];

const ui = { tab: "current", dialog: null, error: null };
let view = null;

const emit = (event) => handle(event);

const header = new Header(emit);
const tabs = new Tabs(emit);
const current = new CurrentRound(emit);
const dialogs = new DialogLayer(emit);
const error = h("p", { class: "error", role: "alert", hidden: true });
const panels = Object.fromEntries(
  TABS.map((tab) => [tab.id, h("section", { class: "panel", role: "tabpanel", "data-panel": tab.id })]),
);
panels.current.appendChild(current.el);

document.body.append(header.el, tabs.el, error, ...Object.values(panels), dialogs.el);

function render() {
  if (!view) return;
  header.update({ title: view.topic.title, original_request: view.topic.original_request });
  tabs.update({ tabs: TABS, current: ui.tab });
  for (const [id, panel] of Object.entries(panels)) panel.hidden = id !== ui.tab;
  current.update(currentRoundData(view));
  error.hidden = !ui.error;
  error.textContent = ui.error ?? "";
  dialogs.update(dialogData());
}

function dialogData() {
  const dialog = ui.dialog;
  if (dialog?.kind === "decision") {
    const detail = decisionDetail(view, dialog.decision);
    return detail ? { kind: "decision", data: detail } : null;
  }
  return null;
}

async function operate(op) {
  const response = await fetch("./api/op", { method: "POST", body: JSON.stringify(op) });
  if (!response.ok) {
    const body = await response.json().catch(() => ({}));
    ui.error = body.error ?? `The server refused the action (${response.status}).`;
  } else {
    ui.error = null;
    accept(await response.json());
  }
  render();
}

/// Keeps the newest view: the answer to an action and the live stream may arrive in either order.
function accept(next) {
  if (!view || next.version >= view.version) view = next;
}

function handle(event) {
  switch (event.type) {
    case "op":
      operate(event.op);
      return;
    case "tab":
      ui.tab = event.tab;
      break;
    case "show-decision":
      ui.dialog = { kind: "decision", decision: event.decision };
      break;
    case "show-finished-picture":
      ui.dialog = { kind: "finished-picture" };
      break;
    case "close-dialog":
      ui.dialog = null;
      break;
    default:
      return;
  }
  render();
}

const stream = new EventSource("./api/events");
stream.addEventListener("view", (message) => {
  accept(JSON.parse(message.data));
  render();
});
