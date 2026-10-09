import { h } from '../dom.js';
import { translator } from '../strings.js';
import { operationIcon } from './operation-icon.js';

export function operationButton(icon, name, attrs = {}) {
  return h('button', { type: 'button', class: 'operation-button', 'aria-label': name, title: name, ...attrs }, operationIcon(icon));
}

export function zoomControls({ lang, scale, out, whole, into, prefix }) {
  const t = translator(lang);
  const percentage = h('span', { class: 'diagram-percentage' });
  const setScale = value => { percentage.textContent = `${Math.round(value * 1000) / 10}%`; };
  setScale(scale);
  const attrs = (action, callback) => ({ 'data-action': `${prefix}-${action}`, 'data-focus': `${prefix}-${action}`, onclick: callback });
  return { setScale, el: h('div', { class: 'diagram-zoom-group', role: 'group', 'aria-label': t('map.zoom') },
    operationButton('minus', t('map.zoom-out'), attrs('zoom-out', out)), percentage,
    operationButton('whole', t('part.whole'), attrs('fit', whole)), operationButton('plus', t('map.zoom-in'), attrs('zoom-in', into))) };
}
