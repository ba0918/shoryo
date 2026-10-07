// The top-level component: holds the server's view and the screen-wide state (the tab and
// the open dialog), turns them into each region's view data, and carries the person's
// actions to the server. No region talks to another; everything passes through here.
import { h } from "./dom.js";
import { CurrentRound } from "./components/current-round.js";
import { DialogLayer } from "./components/dialog.js";
import { Header, Tabs } from "./components/header.js";
import { DecisionsTab } from "./components/decisions.js";
import { MapTab } from "./components/map.js";
import { renderDiagram } from "./diagram.js";
import { PastRounds } from "./components/past-rounds.js";
import { currentRoundData, decisionDetail, decisionsTabData, mapData, pastRoundsData } from "./view-data.js";

const TABS = [
  { id: "current", name: "Current round" },
  { id: "past", name: "Past rounds" },
  { id: "map", name: "Map" },
  { id: "decisions", name: "Decisions" },
];

const ui = {
  tab: "current",
  dialog: null,
  error: null,
  pastRound: null,
  mapRange: "all",
  mapSelected: null,
  /// The question a jump landed on, highlighted until the person moves on.
  landed: null,
  /// Where each jump came from, so "Back" can return there.
  history: [],
};
let view = null;

const emit = (event) => handle(event);

const header = new Header(emit);
const tabs = new Tabs(emit);
const current = new CurrentRound(emit);
const past = new PastRounds(emit);
const decisions = new DecisionsTab(emit);
const ended = h(
  "p",
  { class: "ended", "data-ended": true, hidden: true },
  "This brainstorm has ended. Everything stays readable; nothing more can be sent.",
);
const map = new MapTab(emit);
const dialogs = new DialogLayer(emit, {
  "finished-picture": (picture) => ({
    label: "Finished picture",
    attrs: { "data-finished-picture": true },
    body: [
      h("h3", {}, "Finished picture"),
      picture ? h("div", { class: "diagram-box" }, renderDiagram(picture)) : h("p", { class: "empty" }, "No finished picture yet."),
    ],
  }),
});
const back = h(
  "button",
  { type: "button", class: "back", "data-action": "back", hidden: true, onclick: () => emit({ type: "back" }) },
  "Back",
);
const error = h("p", { class: "error", role: "alert", hidden: true });
const panels = Object.fromEntries(
  TABS.map((tab) => [tab.id, h("section", { class: "panel", role: "tabpanel", "data-panel": tab.id })]),
);
panels.current.appendChild(current.el);
panels.past.appendChild(past.el);
panels.decisions.appendChild(decisions.el);
panels.map.appendChild(map.el);

document.body.append(header.el, ended, tabs.el, back, error, ...Object.values(panels), dialogs.el);

function render() {
  if (!view) return;
  header.update({ title: view.topic.title, original_request: view.topic.original_request });
  tabs.update({ tabs: TABS, current: ui.tab });
  for (const [id, panel] of Object.entries(panels)) panel.hidden = id !== ui.tab;
  ended.hidden = !view.topic.ended;
  back.hidden = ui.history.length === 0;
  current.update(currentRoundData(view, ui.landed));
  past.update(pastRoundsData(view, ui.pastRound, ui.landed));
  map.update(mapData(view, ui.mapRange, ui.mapSelected));
  decisions.update(decisionsTabData(view));
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
  if (dialog?.kind === "finished-picture") {
    return { kind: "finished-picture", data: view.topic.finished_picture };
  }
  return null;
}

// Actions go to the server one at a time, in the order the person made them, so a quick
// "Send all" can never overtake the choice made just before it.
let queue = Promise.resolve();
function enqueue(op) {
  queue = queue.then(() => operate(op)).catch((failure) => {
    ui.error = `The server could not be reached: ${failure.message}`;
    render();
  });
}

async function operate(op) {
  const response = await fetch("./api/op", { method: "POST", body: JSON.stringify(op) });
  if (!response.ok) {
    const body = await response.json().catch(() => ({}));
    ui.error = body.error ?? `The server refused the action (${response.status}).`;
    // The action may have been made on what the screen showed before a change; show the
    // current state with the reason.
    const current = await fetch("./api/view").catch(() => null);
    if (current?.ok) accept(await current.json());
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
      enqueue(event.op);
      return;
    case "tab":
      ui.tab = event.tab;
      ui.landed = null;
      break;
    case "map-range":
      ui.mapRange = event.range;
      break;
    case "select-node":
      ui.mapSelected = event.key;
      break;
    case "jump":
      ui.history.push({ tab: ui.tab, pastRound: ui.pastRound, mapRange: ui.mapRange, mapSelected: ui.mapSelected });
      ui.tab = event.target.tab;
      if (event.target.round !== null) ui.pastRound = event.target.round;
      ui.landed = event.target.question;
      ui.dialog = null;
      render();
      document.querySelector("[data-landed]")?.scrollIntoView({ block: "center" });
      return;
    case "back": {
      const place = ui.history.pop();
      if (place) Object.assign(ui, place, { landed: null });
      break;
    }
    case "choose-round":
      ui.pastRound = event.round;
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
