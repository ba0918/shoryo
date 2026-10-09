export function guardDiagram(part) {
  function count(items, min, max, field) {
    if (!Array.isArray(items) || items.length < min || items.length > max) throw new Error(`${field} count must be ${min}–${max}`);
  }
  function text(value, max, field) {
    if (typeof value !== "string" || !value.trim() || value.length > max * 2 || [...value].length > max) throw new Error(`${field} must have 1–${max} characters`);
  }
  function integer(value, min, max, field) {
    if (!Number.isInteger(value) || value < min || value > max) throw new Error(`${field} must be an integer ${min}–${max}`);
  }
  function point(value, field) {
    integer(value?.x, 0, 8192, `${field}.x`);
    integer(value?.y, 0, 8192, `${field}.y`);
  }
  function ids(items, field) {
    const result = new Set();
    for (const item of items) {
      text(item.id, 64, `${field}.id`);
      if (!/^[A-Za-z][A-Za-z0-9_-]*$/.test(item.id) || result.has(item.id)) throw new Error(`${field} has invalid or duplicate ID ${item.id}`);
      result.add(item.id);
      text(item.label, 512, `${field}.${item.id}.label`);
    }
    return result;
  }
  text(part.title, 120, "title");
  if (!["proposal", "example", "confirmed"].includes(part.role)) throw new Error("Invalid role");
  if (part.canvas !== undefined) {
    integer(part.canvas.width, 64, 8192, "canvas.width");
    integer(part.canvas.height, 64, 8192, "canvas.height");
  }
  if (part.type === "sequence") {
    count(part.participants, 1, 12, "participants");
    count(part.events, 1, 96, "events");
    const known = ids(part.participants, "participant");
    for (const [field, value] of Object.entries(part.layout ?? {})) {
      if (!["participant_gap", "event_gap", "self_loop_width"].includes(field)) throw new Error("Unknown sequence layout field");
      integer(value, 16, 512, `layout.${field}`);
    }
    let previous = -1;
    for (const [i, participant] of part.participants.entries()) {
      if (participant.x !== undefined) integer(participant.x, 0, 8192, "participant.x");
      const x = participant.x ?? 124 + i * (part.layout?.participant_gap ?? 220);
      if (x <= previous) throw new Error("Participant centers must increase");
      previous = x;
    }
    let messages = 0, frames = 0;
    function message(item) {
      if (item.type !== "message" || !["call", "return"].includes(item.kind)) throw new Error("Invalid message kind or nested frame");
      if (!known.has(item.from) || !known.has(item.to)) throw new Error("Unknown message participant");
      text(item.label, 512, "message.label");
      if (item.gap_after !== undefined) integer(item.gap_after, 16, 512, "message.gap_after");
      if (++messages > 96) throw new Error("Total messages exceeds 96");
    }
    for (const event of part.events) {
      if (event.gap_after !== undefined) integer(event.gap_after, 16, 512, "event.gap_after");
      if (event.type === "message") { message(event); continue; }
      if (++frames > 12) throw new Error("Frames exceeds 12");
      if (event.type === "alt") {
        count(event.branches, 2, 8, "branches");
        for (const branch of event.branches) {
          text(branch.condition, 512, "branch.condition");
          count(branch.messages, 1, 96, "branch.messages");
          branch.messages.forEach(message);
        }
      } else if (event.type === "loop") {
        text(event.condition, 512, "loop.condition");
        count(event.messages, 1, 96, "loop.messages");
        event.messages.forEach(message);
      } else throw new Error("Invalid sequence event");
    }
  } else if (part.type === "flow") {
    count(part.nodes, 1, 32, "nodes");
    count(part.edges, 0, 64, "edges");
    const known = ids(part.nodes, "node");
    const decisions = new Set(part.nodes.filter(n => n.kind === "decision").map(n => n.id));
    for (const node of part.nodes) {
      if (!["start", "end", "process", "decision"].includes(node.kind)) throw new Error("Invalid node kind");
      if (node.position !== undefined) point(node.position, "node.position");
      if (node.width !== undefined) integer(node.width, 80, 640, "node.width");
      if (node.height !== undefined) integer(node.height, 32, 640, "node.height");
    }
    for (const edge of part.edges) {
      if (!known.has(edge.from) || !known.has(edge.to)) throw new Error("Unknown edge node");
      for (const port of [edge.from_port, edge.to_port]) if (port !== undefined && !["north", "east", "south", "west"].includes(port)) throw new Error("Invalid edge port");
      if (edge.via !== undefined) { count(edge.via, 0, 12, "edge.via"); edge.via.forEach(p => point(p, "via")); }
      if (decisions.has(edge.from) && !edge.label) throw new Error("Decision edge requires condition");
      if (edge.label !== undefined) {
        text(edge.label.text, 512, "edge.label.text");
        if (edge.label.position !== undefined) point(edge.label.position, "edge.label.position");
      }
    }
  } else throw new Error("Unsupported diagram type");
}
