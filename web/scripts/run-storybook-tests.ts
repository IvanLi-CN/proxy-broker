import { spawn } from "node:child_process";
import process from "node:process";
import { setTimeout as delay } from "node:timers/promises";

const storybookUrl = "http://127.0.0.1:38182";

async function waitForServer(url: string, attempts = 60) {
  for (let attempt = 0; attempt < attempts; attempt += 1) {
    try {
      const response = await fetch(url);
      if (response.ok) {
        return;
      }
    } catch {
      // ignore until ready
    }
    await delay(1_000);
  }
  throw new Error(`Timed out waiting for Storybook at ${url}`);
}

const server = spawn("bun", ["run", "storybook:ci"], {
  stdio: "inherit",
  env: process.env,
});

const stop = async () => {
  if (!server.killed) {
    server.kill("SIGTERM");
  }

  if (server.exitCode !== null) {
    return;
  }

  await new Promise<void>((resolve) => {
    server.once("exit", () => resolve());
    server.once("error", () => resolve());
  });
};

for (const signal of ["SIGINT", "SIGTERM"] as const) {
  process.on(signal, async () => {
    await stop();
    process.exit(1);
  });
}

try {
  await waitForServer(storybookUrl);
  const runner = spawn(
    "bunx",
    [
      "test-storybook",
      "--url",
      storybookUrl,
      "--ci",
      "--browsers",
      "chromium",
      "--failOnConsole",
      "--maxWorkers=2",
      "--testTimeout=120000",
    ],
    {
      stdio: "inherit",
      env: process.env,
    },
  );
  const exitCode = await new Promise<number>((resolve, reject) => {
    runner.once("error", reject);
    runner.once("exit", (code) => resolve(code ?? 1));
  });
  if (exitCode !== 0) {
    throw new Error(`Storybook test-runner failed with exit code ${exitCode}`);
  }
} finally {
  await stop();
}
