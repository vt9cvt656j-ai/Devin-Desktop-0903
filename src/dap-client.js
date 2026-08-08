// Michael IDE — Debug Adapter Protocol client.
//
// The Rust backend (`debug.rs`) spawns the debug adapter and handles
// Content-Length framing, forwarding each decoded DAP message to the frontend
// as JSON. This module is the actual DAP *client*: it drives the
// initialize/launch/configurationDone handshake, mirrors breakpoints, tracks
// stack frames / scopes / variables on stop, streams adapter output to the
// Debug Console, and exposes the run-control commands (continue, step,
// pause, restart, stop) plus expression evaluation for the console and watches.
//
// One active session at a time — enough for a focused single-target debugger,
// and far simpler than VS Code's multi-session tree.

export function createDapManager(options) {
  const {
    backend,
    getWorkspaceRoots = () => [],
    getAllBreakpoints = () => new Map(),
    runInTerminal = null,
    showToast = () => {},
    callbacks = {},
  } = options;

  let session = null;
  let previousStop = Promise.resolve();

  function isActive() {
    return !!session;
  }
  function isStopped() {
    return !!session && session.stopped;
  }
  function capabilities() {
    return session?.capabilities || {};
  }
  function currentThreadId() {
    return session?.threadId ?? null;
  }
  function currentFrames() {
    return session?.frames || [];
  }
  function activeFrameId() {
    return session?.activeFrameId ?? session?.frames?.[0]?.id ?? null;
  }
  function consoleLog() {
    return session?.console || [];
  }

  function emit(name, ...args) {
    try { callbacks[name]?.(...args); } catch { /* never let UI callbacks break the session */ }
  }

  function pushConsole(category, text, target = session) {
    if (!target || session !== target) return;
    target.console.push({ category, text, ts: Date.now() });
    if (target.console.length > 1000) target.console.shift();
    emit("onOutput", category, text);
  }

  // ---- raw protocol I/O ----
  function sendRequest(command, args, target = session) {
    if (!target || session !== target) return Promise.resolve(null);
    const seq = target.seq++;
    const msg = { seq, type: "request", command, arguments: args || {} };
    return new Promise((resolve) => {
      const timer = setTimeout(() => {
        target.pending.delete(seq);
        resolve(null);
      }, 20000);
      target.pending.set(seq, { resolve, timer, command });
      backend.dapSend(target.adapterId, JSON.stringify(msg)).catch(() => {
        const p = target.pending.get(seq);
        if (p) {
          target.pending.delete(seq);
          clearTimeout(p.timer);
          resolve(null);
        }
      });
    });
  }

  function sendResponse(request, body, success = true, message, target = session) {
    if (!target || session !== target) return;
    const msg = {
      seq: target.seq++,
      type: "response",
      request_seq: request.seq,
      success,
      command: request.command,
      body,
      message,
    };
    backend.dapSend(target.adapterId, JSON.stringify(msg)).catch(() => {});
  }

  function onEvent(target, ev) {
    if (!target || session !== target) return;
    if (ev?.kind === "message") {
      let msg;
      try { msg = JSON.parse(ev.data); } catch { return; }
      routeMessage(target, msg);
    } else if (ev?.kind === "stopped" || ev?.kind === "error") {
      // Adapter process died.
      if (ev.kind === "error" && ev.message) pushConsole("stderr", ev.message + "\n", target);
      endSession(ev.kind === "error" ? "adapter error" : "adapter exited", target);
    }
  }

  function routeMessage(target, msg) {
    if (session !== target) return;
    if (msg.type === "response") {
      const pending = target.pending.get(msg.request_seq);
      if (pending) {
        target.pending.delete(msg.request_seq);
        clearTimeout(pending.timer);
        pending.resolve(msg.success ? (msg.body ?? {}) : null);
        if (!msg.success && msg.message) pushConsole("stderr", `[${pending.command}] ${msg.message}\n`, target);
      }
      return;
    }
    if (msg.type === "event") {
      void handleAdapterEvent(target, msg.event, msg.body || {}).catch((error) => {
        if (session === target) pushConsole("stderr", `[adapter event] ${String(error?.message || error)}\n`, target);
      });
      return;
    }
    if (msg.type === "request") {
      handleReverseRequest(target, msg);
    }
  }

  async function handleAdapterEvent(target, event, body) {
    if (session !== target) return;
    switch (event) {
      case "initialized":
        await onInitialized(target);
        break;
      case "output":
        pushConsole(body.category || "console", body.output || "", target);
        break;
      case "stopped":
        await onStopped(target, body);
        break;
      case "continued":
        target.stopped = false;
        target.frames = [];
        target.activeFrameId = null;
        emit("onContinued");
        emit("onState");
        break;
      case "thread":
        emit("onState");
        break;
      case "terminated":
        endSession("terminated", target);
        break;
      case "exited":
        pushConsole("console", `\nProgram exited with code ${body.exitCode ?? 0}.\n`, target);
        break;
      case "breakpoint":
        emit("onState");
        break;
      default:
        break;
    }
  }

  function handleReverseRequest(target, req) {
    if (session !== target) return;
    switch (req.command) {
      case "runInTerminal": {
        const args = req.arguments || {};
        if (runInTerminal) {
          try { runInTerminal(args); } catch { /* ignore */ }
        }
        sendResponse(req, { processId: null, shellProcessId: null }, true, undefined, target);
        break;
      }
      case "startDebugging":
        // Child-session debugging is out of scope; acknowledge so the adapter
        // does not stall.
        sendResponse(req, {}, true, undefined, target);
        break;
      default:
        sendResponse(req, {}, false, `Unsupported reverse request: ${req.command}`, target);
        break;
    }
  }

  // ---- handshake ----
  async function onInitialized(target) {
    // Now that the adapter is ready, push every breakpoint then signal done.
    const all = getAllBreakpoints() || new Map();
    for (const [path, linesSet] of all.entries()) {
      if (session !== target) return;
      const lines = [...linesSet];
      if (lines.length) await sendBreakpoints(path, lines, target);
    }
    if (session !== target) return;
    const filters = (target.capabilities.exceptionBreakpointFilters || [])
      .filter((f) => f.default)
      .map((f) => f.filter);
    await sendRequest("setExceptionBreakpoints", { filters }, target);
    if (session !== target) return;
    await sendRequest("configurationDone", {}, target);
    if (session !== target) return;
    target.configured = true;
    emit("onState");
  }

  async function onStopped(target, body) {
    if (session !== target) return;
    target.stopped = true;
    target.threadId = body.threadId ?? target.threadId;
    target.stopReason = body.reason || "paused";
    if (body.text || body.description) {
      pushConsole("console", `Paused: ${body.description || body.reason}${body.text ? ` — ${body.text}` : ""}\n`, target);
    }
    await refreshStack(target);
    if (session !== target) return;
    const top = target.frames[0];
    if (top && top.source?.path) {
      emit("onShowLocation", top.source.path, top.line, top.column || 1);
    }
    emit("onStopped", { reason: target.stopReason, threadId: target.threadId });
    emit("onState");
  }

  async function refreshStack(target = session) {
    if (!target || session !== target) return;
    if (target.threadId == null) {
      target.frames = [];
      return;
    }
    const body = await sendRequest("stackTrace", { threadId: target.threadId, startFrame: 0, levels: 50 }, target);
    if (session !== target) return;
    target.frames = (body?.stackFrames || []).map((f) => ({
      id: f.id,
      name: f.name,
      line: f.line,
      column: f.column,
      source: f.source || null,
    }));
    target.activeFrameId = target.frames[0]?.id ?? null;
  }

  // ---- public: lifecycle ----
  async function start(config) {
    await previousStop.catch(() => {});
    if (session) {
      showToast("A debug session is already running.");
      return false;
    }
    const adapterId = config.adapterId || config.type;
    const target = {
      adapterId,
      seq: 1,
      pending: new Map(),
      capabilities: {},
      threadId: null,
      stopped: false,
      frames: [],
      activeFrameId: null,
      console: [],
      configured: false,
      config,
    };
    session = target;
    emit("onState");
    pushConsole("console", `Starting debug adapter "${adapterId}"…\n`);

    try {
      await backend.dapStart(
        { adapterId, command: config.command || "", args: config.args || [], cwd: config.cwd || getWorkspaceRoots()[0] || null },
        (event) => onEvent(target, event),
      );
    } catch (e) {
      if (session !== target) return false;
      const msg = String(e && e.message ? e.message : e);
      pushConsole("stderr", `Failed to start adapter: ${msg}\n`, target);
      session = null;
      emit("onState");
      showToast(`Debug adapter failed: ${msg}`);
      return false;
    }

    if (session !== target) return false;
    const initBody = await sendRequest("initialize", {
      clientID: "michael-ide",
      clientName: "Michael IDE",
      adapterID: adapterId,
      locale: "en",
      linesStartAt1: true,
      columnsStartAt1: true,
      pathFormat: "path",
      supportsVariableType: true,
      supportsVariablePaging: false,
      supportsRunInTerminalRequest: true,
      supportsProgressReporting: true,
      supportsInvalidatedEvent: true,
    }, target);
    if (session !== target) return false; // could have died
    if (initBody === null) {
      pushConsole("stderr", "The initialize request timed out or was rejected by the adapter.\n");
      showToast("Debug 初始化失败：调试器没有响应");
      await endSession("initialize failed", target);
      return false;
    }
    target.capabilities = initBody || {};

    // Kick off launch/attach; the adapter replies with an `initialized` event
    // that triggers breakpoint registration + configurationDone.
    const request = config.request === "attach" ? "attach" : "launch";
    const launchArgs = Object.assign(
      { cwd: config.cwd || getWorkspaceRoots()[0], __sessionId: undefined },
      config.launchArgs || {},
    );
    delete launchArgs.__sessionId;
    const ok = await sendRequest(request, launchArgs, target);
    if (session !== target) return false;
    if (ok === null) {
      pushConsole("stderr", `The ${request} request was rejected by the adapter.\n`);
      showToast(`Debug ${request} failed — check the configuration.`);
      await endSession(`${request} failed`, target);
      return false;
    } else {
      showToast(`Debugging started: ${config.name || adapterId}`);
    }
    emit("onState");
    return true;
  }

  function endSession(reason, target = session) {
    if (!target || session !== target) return previousStop;
    const adapterId = target.adapterId;
    for (const { timer, resolve } of target.pending.values()) {
      clearTimeout(timer);
      resolve(null);
    }
    target.pending.clear();
    pushConsole("console", `\nDebug session ended (${reason}).\n`, target);
    const finalConsole = target.console.slice();
    session = null;
    const stopPromise = Promise.resolve(backend.dapStop(adapterId)).catch(() => {});
    previousStop = stopPromise;
    emit("onTerminated", { reason, console: finalConsole });
    emit("onState");
    return stopPromise;
  }

  // ---- public: run control ----
  async function cont() {
    if (!isStopped()) return;
    session.stopped = false;
    emit("onState");
    await sendRequest("continue", { threadId: session.threadId });
  }
  async function next() {
    if (!isStopped()) return;
    await sendRequest("next", { threadId: session.threadId });
  }
  async function stepIn() {
    if (!isStopped()) return;
    await sendRequest("stepIn", { threadId: session.threadId });
  }
  async function stepOut() {
    if (!isStopped()) return;
    await sendRequest("stepOut", { threadId: session.threadId });
  }
  async function pause() {
    if (!session || session.stopped) return;
    await sendRequest("pause", { threadId: session.threadId ?? 1 });
  }
  async function restart() {
    if (!session) return;
    if (session.capabilities.supportsRestartRequest) {
      await sendRequest("restart", {});
    } else {
      const config = session.config;
      await stop();
      await start(config);
    }
  }
  async function stop() {
    const target = session;
    if (!target) return;
    if (target.capabilities.supportsTerminateRequest) {
      await sendRequest("terminate", {}, target);
    }
    if (session !== target) return;
    await sendRequest("disconnect", { terminateDebuggee: true }, target);
    await endSession("disconnected", target);
  }

  // ---- public: breakpoints ----
  async function sendBreakpoints(path, breakpoints, target = session) {
    if (!target || session !== target) return [];
    const bps = Array.isArray(breakpoints) ? breakpoints : [];
    const normalized = bps.map((bp) => {
      if (typeof bp === "number") return { line: bp };
      return {
        line: bp.line,
        ...(bp.condition ? { condition: bp.condition } : {}),
        ...(bp.hitCondition ? { hitCondition: bp.hitCondition } : {}),
        ...(bp.logMessage ? { logMessage: bp.logMessage } : {}),
      };
    });
    const body = await sendRequest("setBreakpoints", {
      source: { path, name: path.split(/[/\\]/).pop() },
      breakpoints: normalized,
      lines: normalized.map((bp) => bp.line),
      sourceModified: false,
    }, target);
    return body?.breakpoints || [];
  }

  const watchExpressions = [];
  function addWatch(expr) {
    if (!watchExpressions.includes(expr)) watchExpressions.push(expr);
  }
  function removeWatch(expr) {
    const idx = watchExpressions.indexOf(expr);
    if (idx >= 0) watchExpressions.splice(idx, 1);
  }
  async function evaluateWatches() {
    if (!isStopped()) return [];
    const frameId = activeFrameId();
    const results = [];
    for (const expr of watchExpressions) {
      const r = await evaluate(expr, frameId, "watch");
      results.push({ expression: expr, value: r?.result || "undefined", type: r?.type || "" });
    }
    return results;
  }

  // ---- public: data inspection ----
  async function setActiveFrame(frameId) {
    if (session) session.activeFrameId = frameId;
    emit("onState");
  }
  async function scopes(frameId) {
    const body = await sendRequest("scopes", { frameId });
    return body?.scopes || [];
  }
  async function variables(variablesReference) {
    const body = await sendRequest("variables", { variablesReference });
    return body?.variables || [];
  }
  async function evaluate(expression, frameId, context = "repl") {
    const body = await sendRequest("evaluate", { expression, frameId, context });
    return body || null;
  }
  async function threads() {
    const body = await sendRequest("threads", {});
    return body?.threads || [];
  }

  return {
    isActive,
    isStopped,
    capabilities,
    currentThreadId,
    currentFrames,
    activeFrameId,
    consoleLog,
    start,
    stop,
    restart,
    cont,
    next,
    stepIn,
    stepOut,
    pause,
    sendBreakpoints,
    setActiveFrame,
    scopes,
    variables,
    evaluate,
    threads,
    addWatch,
    removeWatch,
    evaluateWatches,
    get watchExpressions() { return watchExpressions.slice(); },
  };
}
