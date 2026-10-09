import { Component, h } from '../dom.js';
import { PanZoom, wheelHintKey } from '../pan-zoom.js';
import { translator } from '../strings.js';
import { explanationDiagram } from './explanation-diagram.js';
import { operationIcon } from './operation-icon.js';
import { operationButton, zoomControls } from './diagram-controls.js';

export function explanationFit(box, size) {
  const { width: w, height: h } = box;
  if (!Number.isFinite(w) || !Number.isFinite(h) || w <= 0 || h <= 0) return null;
  const k = Math.min(1, (w - 2 * Math.min(12, w / 4)) / size.width, (h - 2 * Math.min(12, h / 4)) / size.height);
  return { k, x: (w - size.width * k) / 2, y: (h - size.height * k) / 2 };
}

export class DiagramParts {
  constructor(emit) { this.emit = emit; this.items = new Map(); this.shown = new Set(); }
  begin() { this.shown.clear(); }
  render(part, lang, identity) {
    this.shown.add(identity);
    let component = this.items.get(identity);
    if (!component || JSON.stringify(component.data.part) !== JSON.stringify(part)) {
      component?.dispose();
      component = new ExplanationViewport(this.emit);
      this.items.set(identity, component);
    }
    return component.update({ part, lang, identity }).el;
  }
  end() {
    for (const [identity, component] of this.items) if (!this.shown.has(identity)) {
      component.dispose(); this.items.delete(identity);
    }
  }
}

export class ExplanationViewport extends Component {
  constructor(emit, { modal = false } = {}) { super(emit); this.modal = modal; this.fitted = modal; this.menuOpen = false; this.view = { x: 0, y: 0, k: 1 }; }
  dispose() { this.cleanup?.(); }
  draw({ part, lang, identity }) {
    this.dispose();
    const t = translator(lang), lifetime = new AbortController();
    let svg, size, fitting = false, controls;
    const transform = () => { if (svg) svg.style.transform = `translate(${this.view.x}px, ${this.view.y}px) scale(${this.view.k})`; };
    const fit = () => {
      if (!svg || !size) return;
      const view = explanationFit(surface.getBoundingClientRect(), size);
      if (!view) return;
      fitting = true; zoom.set(view, 'now'); fitting = false;
    };
    const whole = () => { this.fitted = true; fit(); };
    const enlarge = h('button', { type: 'button', class: 'operation-button', hidden: true, 'aria-label': t('part.enlarge'), title: t('part.enlarge'), 'data-focus': `enlarge-${identity}`, onclick: () => this.emit({ type: 'show-explanation', part, identity }) }, operationIcon('enlarge'));
    const host = explanationDiagram(part, lang, {
      signal: lifetime.signal,
      onReady: result => {
        svg = result;
        size = { width: Number(result.getAttribute('width')), height: Number(result.getAttribute('height')) };
        if (!this.modal) surface.style.height = `min(${size.height}px, 22.5rem, 50vh)`;
        surface.classList.remove('failed');
        transform();
        if (this.fitted) fit();
        enlarge.hidden = false;
        this.emit?.({ type: 'explanation-ready', identity });
      },
      onFailure: () => { svg = null; enlarge.hidden = true; surface.style.height = ''; surface.classList.add('failed'); zoom.cancelGesture(); },
    });
    const surface = h('div', { class: this.modal ? 'explanation-viewer-surface' : 'explanation-inline-surface', role: 'group', 'aria-label': t(this.modal ? 'part.viewer' : 'part.inline-viewport'), tabindex: '0', 'data-focus': `${this.modal ? 'viewer' : 'inline'}-${identity}`, 'data-explanation-viewer': this.modal ? identity : null }, host);
    const zoom = new PanZoom(surface, this.view, view => { if (!fitting) this.fitted = false; this.view = view; transform(); controls?.setScale(view.k); }, { mode: this.modal ? 'canvas' : 'inline', hint: t(wheelHintKey()) });
    const actions = {
      ArrowLeft: [-40, 0], ArrowRight: [40, 0], ArrowUp: [0, -40], ArrowDown: [0, 40],
    };
    surface.addEventListener('keydown', event => {
      if (event.target !== surface || event.ctrlKey || event.metaKey || event.altKey || !svg) return;
      if (actions[event.key]) {
        event.preventDefault(); const [dx, dy] = actions[event.key];
        zoom.set({ ...zoom.view, x: zoom.view.x + dx, y: zoom.view.y + dy }, 'now');
      } else if (event.key === '+' || event.key === '-') {
        event.preventDefault(); zoom.zoomBy(event.key === '+' ? 1.2 : 1 / 1.2);
      } else if (event.key === '0') {
        event.preventDefault(); whole();
      }
    });
    controls = zoomControls({ lang, scale: this.view.k, out: () => zoom.zoomBy(1 / 1.2), whole, into: () => zoom.zoomBy(1.2), prefix: `explanation-${identity}` });
    const move = (dx, dy) => zoom.set({ ...zoom.view, x: zoom.view.x + dx, y: zoom.view.y + dy }, 'now');
    const popupId = `move-${this.modal ? 'dialog' : 'inline'}-${identity}`;
    const popup = h('div', { id: popupId, class: 'diagram-move-popover', role: 'group', 'aria-label': t('part.move'), hidden: !this.menuOpen },
      Object.entries(actions).map(([key, [dx, dy]]) => h('button', { type: 'button', 'data-focus': `${popupId}-${key}`, onclick: () => move(dx, dy) }, t(`part.${key.slice(5).toLowerCase()}`))));
    const closeMenu = (restore = false) => { this.menuOpen = false; popup.hidden = true; trigger.setAttribute('aria-expanded', 'false'); if (restore) trigger.focus(); };
    const trigger = operationButton('move', t('part.move'), { 'data-focus': `${popupId}-trigger`, 'aria-expanded': String(this.menuOpen), 'aria-controls': popupId, onclick: () => { this.menuOpen = !this.menuOpen; popup.hidden = !this.menuOpen; trigger.setAttribute('aria-expanded', String(this.menuOpen)); } });
    const moveGroup = h('div', { class: 'diagram-move-controls', onkeydown: event => { if (event.key === 'Escape' && this.menuOpen) { event.preventDefault(); event.stopPropagation(); closeMenu(true); } } }, trigger, popup);
    const dismissOutside = event => { if (!moveGroup.contains(event.target)) closeMenu(); };
    document.addEventListener('pointerdown', dismissOutside, { passive: true, signal: lifetime.signal });
    document.addEventListener('click', dismissOutside, { capture: true, passive: true, signal: lifetime.signal });
    surface.addEventListener('focus', () => closeMenu());
    const strip = h('div', { class: 'explanation-viewer-controls' }, this.modal ? controls.el : operationButton('whole', t('part.whole'), { 'data-focus': `whole-${identity}`, onclick: whole }), moveGroup);
    const resize = new ResizeObserver(() => { if (!surface.getClientRects().length) zoom.cancelGesture(); if (this.fitted) fit(); });
    const removal = new MutationObserver(() => {
      if (!surface.isConnected) cleanup();
      else if (!surface.getClientRects().length) zoom.cancelGesture();
    });
    requestAnimationFrame(() => { if (!lifetime.signal.aborted) { resize.observe(surface); removal.observe(document.body, { childList: true, subtree: true, attributes: true, attributeFilter: ['hidden', 'style'] }); } });
    const cleanup = () => { lifetime.abort(); zoom.destroy(); resize.disconnect(); removal.disconnect(); };
    this.cleanup = cleanup;
    const heading = h('header', { class: 'explanation-block-header' }, h('div', { class: 'part-metadata' }, h(this.modal ? 'h2' : 'strong', { class: this.modal ? 'dialog-title' : null }, part.title), h('span', { class: 'badge' }, t(`part.role.${part.role}`))), this.modal ? null : enlarge);
    return h(this.modal ? 'div' : 'section', { class: this.modal ? 'explanation-viewer' : 'explanation-block', 'data-part': this.modal ? null : part.type, onpointerdown: dismissOutside },
      heading, strip, surface, this.modal ? null : h('p', { class: 'explanation-hint' }, t('part.inline-hint')));
  }
}
