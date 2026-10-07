# Diagram text

Read this when writing a diagram: in a reply to an ask (`diagram`) or the finished picture
(`finished_picture`). The screen draws each node exactly at the grid cell you give it and never
re-arranges anything, so the layout is yours.

## Lines

One item per line. Blank lines are ignored.

| Line | Meaning |
|---|---|
| `id = label` | A node. `id` is a short name without spaces; `label` is what the screen shows. |
| `id = ? label` | An empty slot, drawn as a dashed frame. Use it in the finished picture for a place not decided yet. |
| `\| a \| b \| . \|` | One row of the grid, top to bottom. Each cell names a node; `.` is an empty cell. |
| `a -> b : label` | An edge from `a` to `b`. The label is optional (`a -> b`). |

- A node sits in the first cell that names it. A declared node that no row names goes in an
  extra row under the grid.
- A node named in a row but never declared is drawn with its id as the label.
- Two edges between the same two nodes in opposite directions are drawn side by side, not on
  top of each other. Their labels go on opposite sides.
- Keep labels short: a node shows about 18 characters per line and 3 lines at most.

## Example

```text
screen = Screen
server = Local server
file = One JSON file
who = ? Who may read it
| screen | server | file |
| .      | .      | who  |
screen -> server : sends answers
server -> screen : pushes replies
server -> file : saves
file -> who : read by
```

Three nodes in the first row, an empty slot under the file, and a pair of opposite edges
between the screen and the server.
