// Text measured the way the map and the diagrams draw it: an SVG text element with the same
// class, in a hidden SVG of its own, so the browser's own widths decide where a label breaks.
// Character counts cannot: a Japanese character is about twice as wide as a Latin one.

const SVG = "http://www.w3.org/2000/svg";
const ELLIPSIS = "…";

/// One hidden SVG per drawing class, since the map's and the diagrams' text differ in size.
const probes = new Map();
const widths = new Map();

function probeText(svgClass) {
  let probe = probes.get(svgClass);
  if (!probe) {
    const svg = document.createElementNS(SVG, "svg");
    svg.setAttribute("class", svgClass);
    svg.setAttribute("aria-hidden", "true");
    svg.style.cssText = "position:absolute;left:-10000px;top:0;width:0;height:0;visibility:hidden;";
    probe = document.createElementNS(SVG, "text");
    svg.append(probe);
    document.body.append(svg);
    probes.set(svgClass, probe);
  }
  return probe;
}

/// The drawn width of `value` in an SVG text element of class `cls`, inside an SVG of class
/// `svgClass`.
export function textWidth(value, cls, svgClass = "map") {
  const key = `${svgClass}\u0000${cls}\u0000${value}`;
  let width = widths.get(key);
  if (width === undefined) {
    const text = probeText(svgClass);
    text.setAttribute("class", cls);
    text.textContent = value;
    width = text.getComputedTextLength();
    widths.set(key, width);
  }
  return width;
}

/// Breaks `value` into at most `maxLines` lines no wider than `width`, preferring a break at
/// a space; what does not fit is cut with "…" at the end of the last line.
export function fitLines(value, cls, width, maxLines, svgClass = "map") {
  const chars = [...(value ?? "")];
  const fits = (from, to, tail = "") => textWidth(chars.slice(from, to).join("") + tail, cls, svgClass) <= width;
  const lines = [];
  let start = 0;
  while (start < chars.length && lines.length < maxLines) {
    let end = start + 1;
    while (end < chars.length && fits(start, end + 1)) end += 1;
    if (end >= chars.length && fits(start, end)) {
      lines.push(chars.slice(start).join(""));
      break;
    }
    if (lines.length === maxLines - 1) {
      let cut = end;
      while (cut > start && !fits(start, cut, ELLIPSIS)) cut -= 1;
      lines.push(chars.slice(start, cut).join("").trimEnd() + ELLIPSIS);
      break;
    }
    const space = chars.slice(start, end).lastIndexOf(" ");
    const next = space > 0 ? start + space + 1 : end;
    lines.push(chars.slice(start, next).join("").trimEnd());
    start = next;
    while (chars[start] === " ") start += 1;
  }
  return lines;
}
