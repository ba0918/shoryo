// A real shoryo server per test, with its own data and start directories, driven the way the
// agent drives it: through the binary's commands.
import { test as base, expect } from "@playwright/test";
import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
const bin = process.env.SHORYO_BIN ?? path.join(root, "target/debug/shoryo");

function run(dirs, args, input = "") {
  return new Promise((resolve, reject) => {
    const child = spawn(bin, args, { cwd: dirs.work, env: dirs.env });
    let out = "";
    let err = "";
    child.stdout.on("data", (chunk) => (out += chunk));
    child.stderr.on("data", (chunk) => (err += chunk));
    child.on("error", reject);
    child.on("close", (code) => {
      if (code === 0) resolve(out);
      else reject(new Error(`shoryo ${args.join(" ")} failed: ${err}`));
    });
    child.stdin.end(input);
  });
}

function started(child) {
  return new Promise((resolve, reject) => {
    let out = "";
    child.stdout.on("data", (chunk) => {
      out += chunk;
      const url = out.match(/http:\/\/\S+/);
      if (url) resolve(url[0]);
    });
    child.on("error", reject);
    child.on("exit", (code) => reject(new Error(`the server exited with ${code}`)));
  });
}

export const test = base.extend({
  /// The config file's text before the server starts; null leaves no file.
  configText: [null, { option: true }],
  shoryo: async ({ configText }, use) => {
    if (!existsSync(bin)) throw new Error(`${bin} is missing: run scripts/test-web.sh`);
    const scratch = await mkdtemp(path.join(tmpdir(), "shoryo-web-"));
    const dirs = {
      work: scratch,
      env: {
        ...process.env,
        XDG_DATA_HOME: path.join(scratch, ".data"),
        XDG_CONFIG_HOME: path.join(scratch, ".config"),
      },
    };
    if (configText !== null) {
      await mkdir(path.join(scratch, ".config", "shoryo"), { recursive: true });
      await writeFile(path.join(scratch, ".config", "shoryo", "config.toml"), configText);
    }
    const server = spawn(bin, ["start", "topic"], { cwd: dirs.work, env: dirs.env });
    const url = await started(server);
    const shoryo = {
      url,
      round: (body) => run(dirs, ["round", "topic"], JSON.stringify(body)),
      reply: (ask, body) => run(dirs, ["reply", "topic", String(ask)], JSON.stringify(body)),
      end: () => run(dirs, ["end", "topic"]),
      wait: async () => JSON.parse(await run(dirs, ["wait", "topic", "--timeout", "5"])).events,
      op: async (body) => {
        const response = await fetch(`${url}api/op`, { method: "POST", body: JSON.stringify(body) });
        if (!response.ok) throw new Error(`operation refused: ${await response.text()}`);
      },
      /// Stamps every question and sends the current round, as the person does on the screen.
      submit: async () => {
        const view = await (await fetch(`${url}api/view`)).json();
        for (const question of view.unstamped) await shoryo.op({ op: "stamp", question, stamped: true });
        const rounds = view.topic.rounds;
        await shoryo.op({ op: "submit", round: rounds[rounds.length - 1].number });
      },
    };
    await use(shoryo);
    const exited = new Promise((resolve) => server.once("exit", resolve));
    server.kill("SIGTERM");
    const timer = setTimeout(() => server.kill("SIGKILL"), 5000);
    await exited;
    clearTimeout(timer);
    await rm(scratch, { recursive: true, force: true });
  },
});

export { expect };
