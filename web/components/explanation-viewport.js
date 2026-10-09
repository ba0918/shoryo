import { Component, h } from '../dom.js';
import { PanZoom, wheelHintKey } from '../pan-zoom.js';
import { translator } from '../strings.js';
import { explanationDiagram } from './explanation-diagram.js';
import { operationIcon } from './operation-icon.js';

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
  constructor(emit) { super(emit); this.view = { x: 0, y: 0, k: 1 }; }
  dispose() { this.cleanup?.(); }
  draw({ part, lang, identity }) {
    this.dispose();
    const t = translator(lang), lifetime = new AbortController();
    let svg;
    const transform = () => { if (svg) svg.style.transform = `translate(${this.view.x}px, ${this.view.y}px) scale(${this.view.k})`; };
    const enlarge = h('button', { type: 'button', class: 'operation-button', hidden: true, 'aria-label': t('part.enlarge'), title: t('part.enlarge'), 'data-focus': `enlarge-${identity}`, onclick: () => this.emit({ type: 'show-explanation', part, identity }) }, operationIcon('enlarge'));
    const host = explanationDiagram(part, lang, {
      signal: lifetime.signal,
      onReady: result => {
        svg = result;
        surface.style.height = `min(${result.getAttribute('height')}px, 22.5rem, 50vh)`;
        surface.classList.remove('failed');
        transform(); enlarge.hidden = false;
        this.emit({ type: 'explanation-ready', identity });
      },
      onFailure: () => { svg = null; enlarge.hidden = true; surface.style.height = ''; surface.classList.add('failed'); zoom.cancelGesture(); },
    });
    const surface = h('div', { class: 'explanation-inline-surface', role: 'group', 'aria-label': t('part.inline-viewport'), tabindex: '0', 'data-focus': `inline-${identity}` }, host);
    const zoom = new PanZoom(surface, this.view, view => { this.view = view; transform(); }, { mode: 'inline', hint: t(wheelHintKey()) });
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
      }
    });
    const resize = new ResizeObserver(() => { if (!surface.getClientRects().length) zoom.cancelGesture(); });
    const removal = new MutationObserver(() => {
      if (!surface.isConnected) cleanup();
      else if (!surface.getClientRects().length) zoom.cancelGesture();
    });
    requestAnimationFrame(() => { if (!lifetime.signal.aborted) { resize.observe(surface); removal.observe(document.body, { childList: true, subtree: true, attributes: true, attributeFilter: ['hidden', 'style'] }); } });
    const cleanup = () => { lifetime.abort(); zoom.destroy(); resize.disconnect(); removal.disconnect(); };
    this.cleanup = cleanup;
    return h('section', { class: 'explanation-block', 'data-part': part.type },
      h('header', { class: 'explanation-block-header' }, h('div', { class: 'part-metadata' }, h('strong', {}, part.title), h('span', { class: 'badge' }, t(`part.role.${part.role}`))), enlarge),
      surface, h('p', { class: 'explanation-hint' }, t('part.inline-hint')));
  }
}
