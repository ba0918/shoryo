// The top-level component: holds the server's view and the screen-wide state (the tab, the
// open dialog, the language and theme), turns them into each region's view data, and
// carries the person's actions to the server. No region talks to another; everything
// passes through here.
import { h, focusSelector } from "./dom.js";
import { knows, translator } from "./strings.js";
import { CurrentRound } from "./components/current-round.js";
import { DialogLayer } from "./components/dialog.js";
import { Header, Tabs } from "./components/header.js";
import { DecisionsTab } from "./components/decisions.js";
import { DEFAULT_VIEW, MapTab } from "./components/map.js";
import { diagramView } from "./components/diagram-view.js";
import { PastRounds } from "./components/past-rounds.js";
import { Toasts } from "./components/toasts.js";
import { confirmSend } from "./components/confirm-send.js";
import {
  agentStatus,
  arrivalsBetween,
  arrivalText,
  confirmData,
  currentRoundData,
  decisionDetail,
  decisionsTabData,
  mapData,
  pastRoundsData,
  placeReachable,
  reviewConfirmData,
} from "./view-data.js";

const TABS = [{ id: "current" }, { id: "past" }, { id: "map" }, { id: "decisions" }];

/// What the screen shows until the config is read, and when it cannot be.
const DEFAULT_CONFIG = { language: "en", theme: "system", unreadable: false };

const ui = {
  tab: "current",
  dialog: null,
  error: null,
  pastRound: null,
  mapRange: "all",
  mapSelected: null,
  /// The point the 道筋 range leads to; set only by "Show path to this" or the 道筋 button.
  mapRoot: null,
  /// The map's zoom and position; null is the default view.
  mapView: null,
  /// The question a jump landed on, highlighted until the person moves on.
  landed: null,
  /// The places jumps left, newest last, so "Back" can return there (docs/spec/screen.md,
  /// "移動と現在地").
  history: [],
  /// The language and theme, read from the config when the page opens.
  config: null,
  arrivals: [],
  toasts: [],
  arrivalsOpen: false,
  backgroundCount: 0,
};
let pendingArrivals = [];
let arrivalFrame = null;
let toastTimer = null;
let view = null;
/// When the newest view arrived, so the time since the agent was last heard keeps counting.
let viewReceivedAt = 0;
/// Redraws the header when the agent's silence crosses into "not responding".
let agentTimer = null;
/// Where the focus was when the open dialog opened, so closing it can return there.
let dialogOpener = null;

const emit = (event) => handle(event);

const header = new Header(emit);
const toasts = new Toasts(emit);
const tabs = new Tabs(emit);
const current = new CurrentRound(emit);
const past = new PastRounds(emit);
const decisions = new DecisionsTab(emit);
const ended = h("p", { class: "ended", "data-ended": true, hidden: true });
const map = new MapTab(emit);
const dialogs = new DialogLayer(emit, {
  "confirm-send": confirmSend,
  "finished-picture": ({ picture, lang }) => {
    const t = translator(lang);
    return {
      label: t("header.finished-picture"),
      attrs: { "data-finished-picture": true },
      body: [
        h("h2", { class: "dialog-title" }, t("header.finished-picture")),
        picture ? diagramView(picture, lang, "canvas") : h("p", { class: "empty" }, t("dialog.no-picture")),
      ],
    };
  },
});
const error = h("p", { class: "error", role: "alert", hidden: true });
const panels = Object.fromEntries(
  TABS.map((tab) => [tab.id, h("section", { class: "panel", role: "tabpanel", "data-panel": tab.id, "data-focus-scope": `tab-${tab.id}` })]),
);
panels.current.appendChild(current.el);
panels.past.appendChild(past.el);
panels.decisions.appendChild(decisions.el);
panels.map.appendChild(map.el);

const page = h("main", { class: "page" }, ended, h("div", { class: "nav-row" }, tabs.el), error, ...Object.values(panels));
document.body.append(header.el, page, toasts.el, dialogs.el);

function render() {
  if (!view || !ui.config) return;
  const lang = ui.config.language;
  const t = translator(lang);
  const shown = { ...view, lang };
  const agent = agentStatus(view.agent, viewReceivedAt, view.topic, Date.now());
  clearTimeout(agentTimer);
  if (agent?.changesIn != null) agentTimer = setTimeout(render, agent.changesIn);
  const title = view.topic.title || t("header.untitled");
  document.title = document.hidden && ui.backgroundCount ? `(${ui.backgroundCount}) ${title}` : title;
  header.update({
    title: view.topic.title,
    original_request: view.topic.original_request,
    lang,
    theme: ui.config.theme,
    unreadable: ui.config.unreadable,
    canGoBack: ui.history.length > 0,
    agent: agent?.state ?? null,
    arrivals: {
      count: ui.arrivals.filter(arrival => !arrival.seen).length,
      open: ui.arrivalsOpen,
      entries: ui.arrivals.slice(-10).reverse().map(arrival => ({ id: arrival.id, kind: arrival.kind, text: arrivalText(arrival, lang) })),
    },
  });
  tabs.update({ tabs: TABS, current: ui.tab, lang });
  for (const [id, panel] of Object.entries(panels)) panel.hidden = id !== ui.tab;
  ended.hidden = !view.topic.ended;
  ended.textContent = t("screen.ended");
  current.update(currentRoundData(shown, ui.landed), lang);
  past.update(pastRoundsData(shown, ui.pastRound, ui.landed));
  map.update({ ...mapData(shown, ui.mapRange, ui.mapSelected, ui.mapRoot), view: ui.mapView ?? DEFAULT_VIEW, visible: ui.tab === "map" });
  decisions.update(decisionsTabData(shown));
  error.hidden = !ui.error;
  error.textContent = ui.error ? (ui.error.text ?? t(ui.error.key, ui.error.vars)) : "";
  const dialog = dialogData(shown);
  dialogs.update(dialog);
  holdFocusInDialog(dialog !== null);
  toasts.update(ui.toasts.map(id => {
    const arrival = ui.arrivals.find(arrival => arrival.id === id);
    return { id, text: arrivalText(arrival, lang), lang };
  }));
  if (pendingArrivals.length && arrivalFrame === null) {
    arrivalFrame = requestAnimationFrame(() => {
      arrivalFrame = null;
      const pending = pendingArrivals;
      pendingArrivals = [];
      for (const arrival of pending) announce(arrival);
      expireToasts();
      render();
    });
  }
}

function arrivalElement(arrival) {
  const panel = panels[ui.tab];
  if (arrival.kind === "reply") {
    return panel.querySelector(`[data-ask="${CSS.escape(String(arrival.ask))}"] [data-reply]`);
  }
  if (ui.tab !== "current" || view.topic.rounds.at(-1)?.number !== arrival.round) return null;
  return arrival.kind === "result" ? panel.querySelector("[data-result]") :
    [...panel.querySelectorAll("[data-fixes], [data-card], [data-provisional-row], [data-result]")].find(element => element.getBoundingClientRect().height > 0);
}

function arrivalVisible(arrival) {
  const element = arrivalElement(arrival);
  if (!element) return false;
  const box = element.getBoundingClientRect();
  return box.height > 0 && box.width > 0 && box.bottom > header.el.getBoundingClientRect().bottom && box.top < innerHeight && box.right > 0 && box.left < innerWidth;
}

function announce(arrival) {
  const seen = !document.hidden && arrivalVisible(arrival);
  const entry = { ...arrival, seen, deferred: document.hidden, pointer: false, focus: false, expires: Date.now() + 6000 };
  ui.arrivals = [...ui.arrivals, entry];
  if (document.hidden) ui.backgroundCount++;
  else if (!seen) ui.toasts.push(entry.id);
}

function expireToasts() {
  clearTimeout(toastTimer);
  const held = entry => entry.pointer || entry.focus;
  ui.toasts = ui.toasts.filter(id => {
    const entry = ui.arrivals.find(entry => entry.id === id);
    return held(entry) || entry.expires > Date.now();
  });
  while (ui.toasts.length > 3) {
    const index = ui.toasts.slice(0, -1).findIndex(id => !held(ui.arrivals.find(entry => entry.id === id)));
    if (index < 0) break;
    ui.toasts.splice(index, 1);
  }
  const deadlines = ui.toasts.map(id => ui.arrivals.find(entry => entry.id === id)).filter(entry => !held(entry)).map(entry => entry.expires);
  if (deadlines.length) toastTimer = setTimeout(() => { expireToasts(); render(); }, Math.max(0, Math.min(...deadlines) - Date.now()));
}

function viewArrival(id) {
  const arrival = ui.arrivals.find(entry => entry.id === id);
  if (!arrival) return;
  remember();
  arrival.seen = true;
  ui.toasts = ui.toasts.filter(entry => entry !== id);
  ui.arrivalsOpen = false;
  ui.dialog = null;
  const round = view.topic.rounds.find(round => round.number === arrival.round);
  ui.tab = round?.submitted ? "past" : "current";
  if (ui.tab === "past") ui.pastRound = arrival.round;
  ui.landed = arrival.question ?? null;
  render();
  if (arrival.kind === "reply") (ui.tab === "past" ? past : current).revealReply(arrival.question);
  const element = arrivalElement(arrival) ?? panels[ui.tab].querySelector("[data-result], [data-past-round]");
  element?.scrollIntoView({ block: "center" });
  expireToasts();
}

document.addEventListener("visibilitychange", () => {
  if (!document.hidden) {
    ui.backgroundCount = 0;
    for (const entry of ui.arrivals.filter(entry => entry.deferred)) {
      entry.deferred = false;
      entry.seen = arrivalVisible(entry);
      if (!entry.seen) {
        entry.expires = Date.now() + 6000;
        ui.toasts.push(entry.id);
      }
    }
    expireToasts();
  }
  render();
});

/// While a dialog is open, the rest of the page cannot be reached and the focus starts on the
/// dialog's first control; closing it returns the focus to what opened it.
function holdFocusInDialog(open) {
  const wasOpen = dialogOpener !== null;
  for (const part of document.body.children) part.inert = open && part !== dialogs.el;
  if (open && !wasOpen) {
    dialogOpener = focusKey(document.activeElement);
    dialogs.el.querySelector("button, [href], input, select, textarea, [tabindex]")?.focus();
  } else if (!open && wasOpen) {
    const opener = dialogOpener;
    dialogOpener = null;
    if (opener.element.isConnected) opener.element.focus();
    else if (opener.selector) document.querySelector(opener.selector)?.focus();
  }
}

/// An element and a way to find its redrawn replacement.
function focusKey(element) {
  const selector = element ? focusSelector(element) : null;
  return { element: element ?? document.body, selector };
}

function dialogData(shown) {
  const dialog = ui.dialog;
  if (dialog?.kind === "decision") {
    const detail = decisionDetail(shown, dialog.decision);
    return detail ? { kind: "decision", data: detail, lang: shown.lang } : null;
  }
  if (dialog?.kind === "confirm-send") {
    return { kind: "confirm-send", data: confirmData(shown), lang: shown.lang };
  }
  if (dialog?.kind === "confirm-review") {
    return { kind: "confirm-review", data: reviewConfirmData(shown, dialog.decision, dialog.resultWording), lang: shown.lang };
  }
  if (dialog?.kind === "finished-picture") {
    return { kind: "finished-picture", data: { picture: view.topic.finished_picture, lang: shown.lang }, lang: shown.lang };
  }
  return null;
}

/// Puts the theme on the root element, where the colour tokens switch; "system" leaves it
/// to the OS's setting.
function applyConfig() {
  const root = document.documentElement;
  if (ui.config.theme === "system") delete root.dataset.theme;
  else root.dataset.theme = ui.config.theme;
  root.lang = ui.config.language;
}

async function loadConfig() {
  try {
    const response = await fetch("./api/config");
    ui.config = response.ok ? await response.json() : { ...DEFAULT_CONFIG, unreadable: true };
  } catch {
    ui.config = { ...DEFAULT_CONFIG, unreadable: true };
  }
  applyConfig();
  render();
}

/// A switch takes effect on this page at once; it is kept in the config file unless the file
/// cannot be read, and a failed write leaves it for this page only.
let configQueue = Promise.resolve();
function changeConfig(change) {
  ui.config = { ...ui.config, ...change };
  applyConfig();
  render();
  if (ui.config.unreadable) return;
  configQueue = configQueue.then(async () => {
    const response = await fetch("./api/config", { method: "POST", body: JSON.stringify(change) }).catch(() => null);
    if (!response?.ok) {
      ui.config = { ...ui.config, unreadable: true };
      ui.error = { key: "screen.config-not-saved" };
      render();
    }
  });
}

// Actions go to the server one at a time, in the order the person made them, so a quick
// "Send all" can never overtake the choice made just before it.
let queue = Promise.resolve();
function enqueue(op) {
  queue = queue.then(() => operate(op)).catch((failure) => {
    ui.error = { key: "screen.unreachable", vars: { reason: failure.message } };
    render();
  });
}

async function operate(op) {
  const response = await fetch("./api/op", { method: "POST", body: JSON.stringify(op) });
  if (!response.ok) {
    const body = await response.json().catch(() => ({}));
    ui.error = refusalError(body) ?? { key: "screen.refused", vars: { status: response.status } };
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

/// A refusal in the screen's language when its kind is known; otherwise the server's words.
function refusalError(body) {
  const refusal = body.refusal;
  const key = refusal ? `refusal.${refusal.kind}` : null;
  if (key && knows(key)) return { key, vars: { ...refusal, questions: refusal.questions?.join(", ") } };
  return body.error ? { text: body.error } : null;
}

/// Keeps the newest view: the answer to an action and the live stream may arrive in either order.
function accept(next) {
  if (!view || next.version >= view.version) {
    pendingArrivals.push(...arrivalsBetween(view, next));
    view = next;
    viewReceivedAt = Date.now();
    // A place whose card went with its round can no longer be returned to.
    ui.history = ui.history.filter((place) => placeReachable(view, place));
  }
}

/// What a place is anchored on: the card, provisional row, decision or result nearest the
/// top of the page below the header, and how far from the top it was.
const ANCHORS = [
  ["[data-card]", "question", "card"],
  ["[data-provisional-row]", "question", "provisionalRow"],
  ["[data-decision-item]", "decision", "decisionItem"],
  ["[data-result]", "result", null],
];

function anchorNow() {
  const top = header.el.getBoundingClientRect().bottom;
  const panel = panels[ui.tab];
  const candidates = panel.querySelectorAll(ANCHORS.map(([selector]) => selector).join(","));
  for (const element of candidates) {
    const box = element.getBoundingClientRect();
    if (box.height === 0 || box.bottom <= top) continue;
    const [, kind, field] = ANCHORS.find(([selector]) => element.matches(selector));
    return { kind, key: field ? element.dataset[field] : null, offset: box.top };
  }
  return null;
}

function anchorElement(anchor) {
  const panel = panels[ui.tab];
  const key = anchor.key === null ? null : CSS.escape(anchor.key);
  switch (anchor.kind) {
    case "question":
      return panel.querySelector(`[data-card="${key}"], [data-provisional-row="${key}"]`);
    case "decision":
      return panel.querySelector(`[data-decision-item="${key}"]`);
    default:
      return panel.querySelector("[data-result]");
  }
}

/// Keeps where the person is, for "Back": the tab, the past round, the map's range,
/// selection and view, the round the current tab showed, and what the page was scrolled to.
function remember() {
  ui.history.push({
    tab: ui.tab,
    pastRound: ui.pastRound,
    mapRange: ui.mapRange,
    mapSelected: ui.mapSelected,
    mapRoot: ui.mapRoot,
    mapView: ui.mapView,
    currentRound: view.topic.rounds[view.topic.rounds.length - 1]?.number ?? null,
    anchor: anchorNow(),
  });
}

function goBack() {
  const place = ui.history.pop();
  if (!place) return;
  const { anchor, currentRound: _round, ...shown } = place;
  Object.assign(ui, shown, { landed: null });
  render();
  const element = anchor ? anchorElement(anchor) : null;
  if (element) window.scrollBy(0, element.getBoundingClientRect().top - anchor.offset);
}

function handle(event) {
  switch (event.type) {
    case "toggle-arrivals":
      ui.arrivalsOpen = !ui.arrivalsOpen;
      if (ui.arrivalsOpen) ui.arrivals.forEach(entry => { entry.seen = true; });
      break;
    case "view-arrival":
      viewArrival(event.id);
      return;
    case "hold-toast": {
      const entry = ui.arrivals.find(entry => entry.id === event.id);
      if (!entry) return;
      entry[event.kind] = event.held;
      // Removing a focused toast emits blur during removal; finish that DOM change first.
      if (!event.held) queueMicrotask(() => { expireToasts(); render(); });
      return;
    }
    case "op":
      enqueue(event.op);
      return;
    case "tab":
      ui.tab = event.tab;
      ui.landed = null;
      break;
    case "map-range":
      if (event.range === ui.mapRange) return;
      ui.mapRange = event.range;
      // The 道筋 button leads to the selected point, or back to the last root.
      if (event.range === "path") ui.mapRoot = ui.mapSelected ?? ui.mapRoot;
      break;
    case "show-path":
      if (ui.mapRange === "path" && ui.mapRoot === event.key) return;
      ui.mapRange = "path";
      ui.mapRoot = event.key;
      ui.mapSelected = event.key;
      break;
    case "select-node":
      if (event.key === ui.mapSelected) return;
      ui.mapSelected = event.key;
      break;
    case "map-view":
      ui.mapView = event.view;
      break;
    case "jump":
      remember();
      ui.tab = event.target.tab;
      if (event.target.round !== null) ui.pastRound = event.target.round;
      ui.landed = event.target.question;
      ui.dialog = null;
      render();
      document.querySelector("[data-landed]")?.scrollIntoView({ block: "center" });
      return;
    case "back":
      goBack();
      return;
    case "choose-round":
      ui.pastRound = event.round;
      break;
    case "show-decision":
      ui.dialog = { kind: "decision", decision: event.decision };
      break;
    case "show-finished-picture":
      ui.dialog = { kind: "finished-picture" };
      break;
    case "config":
      changeConfig(event.change);
      return;
    case "confirm-send":
      ui.dialog = { kind: "confirm-send" };
      break;
    case "send":
      ui.dialog = null;
      enqueue({ op: "submit", round: event.round });
      break;
    case "go-unstamped":
      remember();
      ui.tab = "current";
      ui.landed = view.unstamped[0] ?? null;
      render();
      (document.querySelector("[data-landed] [data-action=stamp]") ?? document.querySelector("[data-landed]"))?.scrollIntoView({
        block: "center",
      });
      return;
    case "confirm-review":
      ui.dialog = { kind: "confirm-review", decision: event.decision, resultWording: event.resultWording };
      break;
    case "review":
      ui.dialog = null;
      enqueue({ op: "request_review", decision: event.decision });
      break;
    case "close-dialog":
      ui.dialog = null;
      break;
    default:
      return;
  }
  render();
}

// Escape closes the open dialog, the way its close button does.
document.addEventListener("keydown", (event) => {
  if (event.key === "Escape" && ui.dialog) handle({ type: "close-dialog" });
});

loadConfig();
const stream = new EventSource("./api/events");
stream.addEventListener("view", (message) => {
  accept(JSON.parse(message.data));
  render();
});
