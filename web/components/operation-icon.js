const paths = {
  copy: 'M9 9H20V20H9Z M15 9V4H4V15H9',
  success: 'M5 12L10 17L19 7',
  enlarge: 'M14 4H20V10 M20 4L13 11 M10 20H4V14 M4 20L11 13',
  minus: 'M5 12H19',
  plus: 'M5 12H19 M12 5V19',
  whole: 'M9 4H4V9 M15 4H20V9 M4 15V20H9 M20 15V20H15',
  close: 'M6 6L18 18 M18 6L6 18',
  move: 'M12 3V21 M3 12H21 M9 6L12 3L15 6 M9 18L12 21L15 18 M6 9L3 12L6 15 M18 9L21 12L18 15',
};

export function operationIcon(name) {
  const ns = 'http://www.w3.org/2000/svg';
  const svg = document.createElementNS(ns, 'svg');
  for (const [key, value] of Object.entries({ viewBox: '0 0 24 24', width: 18, height: 18, fill: 'none', stroke: 'currentColor', 'stroke-width': 1.65, 'stroke-linecap': 'round', 'stroke-linejoin': 'round', 'aria-hidden': 'true', focusable: 'false' })) svg.setAttribute(key, value);
  const path = document.createElementNS(ns, 'path');
  path.setAttribute('d', paths[name]);
  svg.append(path);
  return svg;
}
