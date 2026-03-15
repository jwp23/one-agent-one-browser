import fs from "node:fs/promises";
import path from "node:path";
import process from "node:process";
import { pathToFileURL } from "node:url";
import { chromium, firefox, webkit } from "playwright";

const HELP_TEXT = `Usage:
  node tools/oracle/oracle.mjs --target <path-or-url> --out-dir <dir> [options]

Required:
  --target <path-or-url>         Local HTML path or http(s)/file URL to capture
  --out-dir <dir>                Directory for oracle artifacts

Options:
  --browser <name>               chromium | firefox | webkit (default: chromium)
  --width <px>                   Viewport width in CSS px (default: 1366)
  --height <px>                  Viewport height in CSS px (default: 768)
  --wait-until <mode>            load | domcontentloaded | networkidle | commit
                                 (default: load)
  --delay-ms <ms>                Extra delay after load and font readiness
                                 (default: 0)
  --timeout-ms <ms>              Timeout for navigation and waits (default: 30000)
  --full-page                    Capture the full scrollable page
  --executable-path <path>       Override browser executable
  --help                         Show this help text
`;

async function main() {
  const options = parseArgs(process.argv.slice(2));
  if (options.help) {
    process.stdout.write(HELP_TEXT);
    return;
  }

  const resolvedTarget = resolveTarget(options.target);
  const outDir = path.resolve(process.cwd(), options.outDir);

  await fs.mkdir(outDir, { recursive: true });

  const browserType = selectBrowserType(options.browser);
  const browser = await browserType.launch({
    executablePath: options.executablePath || undefined,
    headless: true,
  });

  const context = await browser.newContext({
    viewport: { width: options.width, height: options.height },
    deviceScaleFactor: 1,
  });

  const page = await context.newPage();
  page.setDefaultNavigationTimeout(options.timeoutMs);
  page.setDefaultTimeout(options.timeoutMs);

  const events = {
    console: [],
    pageErrors: [],
    requestFailures: [],
  };

  page.on("console", (message) => {
    events.console.push({
      type: message.type(),
      text: message.text(),
      location: message.location(),
    });
  });
  page.on("pageerror", (error) => {
    events.pageErrors.push({
      message: error.message,
      stack: error.stack ?? "",
    });
  });
  page.on("requestfailed", (request) => {
    events.requestFailures.push({
      url: request.url(),
      method: request.method(),
      resourceType: request.resourceType(),
      failureText: request.failure()?.errorText ?? "unknown",
    });
  });

  try {
    await page.goto(resolvedTarget, { waitUntil: options.waitUntil });
    await page.waitForLoadState("load", { timeout: options.timeoutMs }).catch(() => {});
    await waitForFontsAndPaint(page);
    if (options.delayMs > 0) {
      await page.waitForTimeout(options.delayMs);
    }

    const domHtml = await page.content();
    const pageSnapshot = await page.evaluate(() => {
      const docEl = document.documentElement;
      const body = document.body;
      return {
        finalUrl: window.location.href,
        title: document.title,
        readyState: document.readyState,
        htmlClasses: Array.from(docEl?.classList ?? []),
        bodyText: body?.innerText ?? "",
        scrollWidth: docEl?.scrollWidth ?? 0,
        scrollHeight: docEl?.scrollHeight ?? 0,
        userAgent: navigator.userAgent,
      };
    });

    const screenshotPath = path.join(outDir, "screenshot.png");
    await page.screenshot({
      path: screenshotPath,
      fullPage: options.fullPage,
    });

    const manifest = {
      inputTarget: options.target,
      resolvedTarget,
      capturedAt: new Date().toISOString(),
      browser: options.browser,
      viewport: {
        width: options.width,
        height: options.height,
      },
      waitUntil: options.waitUntil,
      delayMs: options.delayMs,
      timeoutMs: options.timeoutMs,
      fullPage: options.fullPage,
      page: {
        url: pageSnapshot.finalUrl,
        title: pageSnapshot.title,
        readyState: pageSnapshot.readyState,
        htmlClasses: pageSnapshot.htmlClasses,
        scrollWidth: pageSnapshot.scrollWidth,
        scrollHeight: pageSnapshot.scrollHeight,
        userAgent: pageSnapshot.userAgent,
      },
      artifacts: {
        screenshot: "screenshot.png",
        dom: "dom.html",
        text: "text.txt",
        events: "events.json",
      },
      eventCounts: {
        console: events.console.length,
        pageErrors: events.pageErrors.length,
        requestFailures: events.requestFailures.length,
      },
    };

    await Promise.all([
      fs.writeFile(path.join(outDir, "dom.html"), domHtml, "utf8"),
      fs.writeFile(path.join(outDir, "text.txt"), pageSnapshot.bodyText, "utf8"),
      fs.writeFile(path.join(outDir, "events.json"), `${JSON.stringify(events, null, 2)}\n`, "utf8"),
      fs.writeFile(
        path.join(outDir, "manifest.json"),
        `${JSON.stringify(manifest, null, 2)}\n`,
        "utf8",
      ),
    ]);
  } finally {
    await context.close();
    await browser.close();
  }
}

function parseArgs(argv) {
  const options = {
    target: "",
    outDir: "",
    browser: "chromium",
    width: 1366,
    height: 768,
    waitUntil: "load",
    delayMs: 0,
    timeoutMs: 30000,
    fullPage: false,
    executablePath: "",
    help: false,
  };

  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    switch (arg) {
      case "--target":
        options.target = readValue(argv, ++index, "--target");
        break;
      case "--out-dir":
        options.outDir = readValue(argv, ++index, "--out-dir");
        break;
      case "--browser":
        options.browser = readValue(argv, ++index, "--browser");
        break;
      case "--width":
        options.width = parseInteger(readValue(argv, ++index, "--width"), "--width");
        break;
      case "--height":
        options.height = parseInteger(readValue(argv, ++index, "--height"), "--height");
        break;
      case "--wait-until":
        options.waitUntil = readValue(argv, ++index, "--wait-until");
        break;
      case "--delay-ms":
        options.delayMs = parseInteger(readValue(argv, ++index, "--delay-ms"), "--delay-ms");
        break;
      case "--timeout-ms":
        options.timeoutMs = parseInteger(readValue(argv, ++index, "--timeout-ms"), "--timeout-ms");
        break;
      case "--executable-path":
        options.executablePath = readValue(argv, ++index, "--executable-path");
        break;
      case "--full-page":
        options.fullPage = true;
        break;
      case "--help":
      case "-h":
        options.help = true;
        break;
      default:
        throw new Error(`Unknown argument: ${arg}`);
    }
  }

  if (!options.help) {
    if (!options.target) {
      throw new Error("Missing required argument: --target");
    }
    if (!options.outDir) {
      throw new Error("Missing required argument: --out-dir");
    }
    if (!["chromium", "firefox", "webkit"].includes(options.browser)) {
      throw new Error(`Unsupported browser: ${options.browser}`);
    }
    if (!["load", "domcontentloaded", "networkidle", "commit"].includes(options.waitUntil)) {
      throw new Error(`Unsupported --wait-until value: ${options.waitUntil}`);
    }
    if (options.width <= 0 || options.height <= 0) {
      throw new Error("Viewport dimensions must be positive");
    }
    if (options.delayMs < 0 || options.timeoutMs <= 0) {
      throw new Error("Timeout and delay values must be non-negative, with timeout > 0");
    }
  }

  return options;
}

function readValue(argv, index, flagName) {
  const value = argv[index];
  if (!value) {
    throw new Error(`Missing value for ${flagName}`);
  }
  return value;
}

function parseInteger(rawValue, flagName) {
  if (!/^-?\d+$/.test(rawValue)) {
    throw new Error(`Invalid integer for ${flagName}: ${rawValue}`);
  }
  const value = Number(rawValue);
  if (!Number.isSafeInteger(value)) {
    throw new Error(`Invalid integer for ${flagName}: ${rawValue}`);
  }
  return value;
}

function resolveTarget(target) {
  if (/^(https?|file):\/\//i.test(target)) {
    return target;
  }

  const absolutePath = path.resolve(process.cwd(), target);
  return pathToFileURL(absolutePath).href;
}

function selectBrowserType(name) {
  switch (name) {
    case "chromium":
      return chromium;
    case "firefox":
      return firefox;
    case "webkit":
      return webkit;
    default:
      throw new Error(`Unsupported browser: ${name}`);
  }
}

async function waitForFontsAndPaint(page) {
  await page.evaluate(async () => {
    if ("fonts" in document) {
      try {
        await document.fonts.ready;
      } catch {
      }
    }

    await new Promise((resolve) => {
      requestAnimationFrame(() => {
        requestAnimationFrame(resolve);
      });
    });
  });
}

main().catch((error) => {
  process.stderr.write(`${error.stack ?? error.message}\n`);
  process.exitCode = 1;
});
