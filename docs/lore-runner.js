#!/usr/bin/env node

const http = require("http");
const { spawn } = require("child_process");
const fs = require("fs");
const path = require("path");

const HOST = "127.0.0.1";
const PORT = 7878;
const MAX_OUTPUT_BYTES = 256 * 1024;
const COMMAND_TIMEOUT_MS = 30_000;

const ALLOWED_SUBCOMMANDS = new Set([
  "init",
  "install",
  "status",
  "context",
  "explain",
  "validate",
  "mcp",
  "propose",
  "signal",
  "checkpoint",
  "set-state",
  "commit",
  "sync",
  "merge",
]);

function writeJson(res, statusCode, payload) {
  const body = JSON.stringify(payload, null, 2);
  res.writeHead(statusCode, {
    "Content-Type": "application/json; charset=utf-8",
    "Access-Control-Allow-Origin": "*",
    "Access-Control-Allow-Methods": "GET, POST, OPTIONS",
    "Access-Control-Allow-Headers": "Content-Type",
  });
  res.end(body);
}

function tokenizeCommand(input) {
  const text = String(input || "").trim();
  if (!text) {
    throw new Error("Comando vacío");
  }
  if (/[\n\r;]/.test(text)) {
    throw new Error("Caracteres no permitidos en comando");
  }

  const tokens = [];
  let current = "";
  let quote = null;

  for (let i = 0; i < text.length; i += 1) {
    const char = text[i];

    if (quote) {
      if (char === quote) {
        quote = null;
      } else if (char === "\\" && i + 1 < text.length) {
        i += 1;
        current += text[i];
      } else {
        current += char;
      }
      continue;
    }

    if (char === '"' || char === "'") {
      quote = char;
      continue;
    }

    if (/\s/.test(char)) {
      if (current) {
        tokens.push(current);
        current = "";
      }
      continue;
    }

    if (/[|&`><]/.test(char)) {
      throw new Error("Operadores de shell no permitidos");
    }

    current += char;
  }

  if (quote) {
    throw new Error("Comando con comillas sin cerrar");
  }
  if (current) {
    tokens.push(current);
  }

  return tokens;
}

function validateCommand(tokens) {
  if (!tokens.length) {
    throw new Error("Comando vacío");
  }

  const executable = tokens[0];
  if (executable !== "git-lore") {
    throw new Error("Solo se permite ejecutar comandos git-lore");
  }

  const subcommand = tokens[1];
  if (!subcommand) {
    throw new Error("Debes indicar un subcomando de git-lore");
  }

  if (subcommand.startsWith("-")) {
    return;
  }

  if (!ALLOWED_SUBCOMMANDS.has(subcommand)) {
    throw new Error("Subcomando no permitido: " + subcommand);
  }
}

function resolveGitLoreExecutable() {
  const localDebug = path.join(process.cwd(), "target", "debug", "git-lore");
  if (fs.existsSync(localDebug)) {
    return localDebug;
  }
  return "git-lore";
}

function executeCommand(tokens) {
  return new Promise((resolve) => {
    const start = Date.now();
    const command = resolveGitLoreExecutable();
    const args = tokens.slice(1);
    const child = spawn(command, args, {
      cwd: process.cwd(),
      env: process.env,
      stdio: ["ignore", "pipe", "pipe"],
    });

    let stdout = "";
    let stderr = "";

    const timer = setTimeout(() => {
      child.kill("SIGTERM");
    }, COMMAND_TIMEOUT_MS);

    child.stdout.on("data", (chunk) => {
      if (stdout.length < MAX_OUTPUT_BYTES) {
        stdout += String(chunk);
      }
    });

    child.stderr.on("data", (chunk) => {
      if (stderr.length < MAX_OUTPUT_BYTES) {
        stderr += String(chunk);
      }
    });

    child.on("error", (error) => {
      clearTimeout(timer);
      resolve({
        ok: false,
        exitCode: -1,
        durationMs: Date.now() - start,
        stdout,
        stderr,
        error: error.message,
      });
    });

    child.on("close", (code) => {
      clearTimeout(timer);
      resolve({
        ok: code === 0,
        exitCode: code,
        durationMs: Date.now() - start,
        stdout,
        stderr,
      });
    });
  });
}

async function readRequestJson(req) {
  return new Promise((resolve, reject) => {
    let body = "";
    req.on("data", (chunk) => {
      body += chunk;
      if (body.length > MAX_OUTPUT_BYTES) {
        reject(new Error("Payload demasiado grande"));
      }
    });
    req.on("end", () => {
      if (!body.trim()) {
        resolve({});
        return;
      }
      try {
        resolve(JSON.parse(body));
      } catch (_) {
        reject(new Error("JSON inválido"));
      }
    });
    req.on("error", reject);
  });
}

const server = http.createServer(async (req, res) => {
  if (req.method === "OPTIONS") {
    writeJson(res, 200, { ok: true });
    return;
  }

  if (req.method === "GET" && req.url === "/health") {
    writeJson(res, 200, {
      ok: true,
      runner: "git-lore-local-runner",
      cwd: process.cwd(),
      executable: resolveGitLoreExecutable(),
      allowedSubcommands: Array.from(ALLOWED_SUBCOMMANDS.values()),
    });
    return;
  }

  if (req.method === "POST" && req.url === "/run") {
    try {
      const payload = await readRequestJson(req);
      const commandText = String(payload.command || "").trim();
      const tokens = tokenizeCommand(commandText);
      validateCommand(tokens);
      const result = await executeCommand(tokens);
      writeJson(res, result.ok ? 200 : 400, result);
    } catch (error) {
      writeJson(res, 400, {
        ok: false,
        exitCode: -1,
        durationMs: 0,
        stdout: "",
        stderr: "",
        error: error.message || String(error),
      });
    }
    return;
  }

  writeJson(res, 404, {
    ok: false,
    error: "Ruta no encontrada",
  });
});

server.listen(PORT, HOST, () => {
  process.stdout.write("git-lore runner listening on http://" + HOST + ":" + PORT + "\n");
});
