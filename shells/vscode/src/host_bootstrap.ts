import { spawn } from "node:child_process";
import type { Readable } from "node:stream";

import {
  AuthenticatedLinuxHostBridge,
  type LinuxHostLaunchCredentials,
} from "./host_bridge.js";
import { UnavailableHostBridge, type HostBridge } from "./provider.js";

const INSTALLED_LINUX_HOST = "/usr/libexec/agentmage/agentmage-host";
const BOOTSTRAP_ARGUMENT = "--bootstrap-linux";
const BOOTSTRAP_MAGIC = Buffer.from("AGMB", "ascii");
const BOOTSTRAP_VERSION = 1;
const BOOTSTRAP_HEADER_BYTES = 8;
const BOOTSTRAP_FIXED_BYTES = 32 + 32 + 4 + 4 + 8 + 32;
const MAX_ENDPOINT_BYTES = 256;
const MAX_BOOTSTRAP_BYTES =
  BOOTSTRAP_HEADER_BYTES + MAX_ENDPOINT_BYTES + BOOTSTRAP_FIXED_BYTES;
const BOOTSTRAP_TIMEOUT_MS = 3_000;

interface HostChild {
  readonly stdout: Readable;
  readonly stderr: Readable;
  once(event: "error" | "exit", listener: () => void): this;
  off(event: "error" | "exit", listener: () => void): this;
  kill(signal?: NodeJS.Signals): boolean;
}

export type InstalledHostLauncher = () => HostChild;

/** Starts only the fixed package host and otherwise returns the inert bridge. */
export async function launchInstalledHost(): Promise<HostBridge> {
  if (process.platform !== "linux" || process.getuid === undefined) {
    return new UnavailableHostBridge();
  }
  return launchWith(
    () =>
      spawn(INSTALLED_LINUX_HOST, [BOOTSTRAP_ARGUMENT], {
        cwd: "/",
        env: { LANG: "C", LC_ALL: "C" },
        shell: false,
        windowsHide: true,
        stdio: ["ignore", "pipe", "pipe"],
      }),
    BOOTSTRAP_TIMEOUT_MS,
  );
}

/** Testable closed launcher boundary; failures never expose process detail. */
export async function launchWith(
  launcher: InstalledHostLauncher,
  timeoutMs: number,
): Promise<HostBridge> {
  if (
    process.platform !== "linux" ||
    process.getuid === undefined ||
    !Number.isSafeInteger(timeoutMs) ||
    timeoutMs <= 0 ||
    timeoutMs > BOOTSTRAP_TIMEOUT_MS
  ) {
    return new UnavailableHostBridge();
  }
  let child: HostChild;
  try {
    child = launcher();
  } catch {
    return new UnavailableHostBridge();
  }
  try {
    const frame = await readBootstrapFrame(child, timeoutMs);
    try {
      const credentials = parseBootstrapFrame(frame, process.getuid());
      if (credentials.peer.pid !== process.pid) {
        credentials.launchSecret.fill(0);
        throw new BootstrapFailure();
      }
      const bridge = new AuthenticatedLinuxHostBridge(credentials);
      return new SupervisedHostBridge(bridge, child);
    } finally {
      frame.fill(0);
    }
  } catch {
    child.kill("SIGKILL");
    return new UnavailableHostBridge();
  }
}

class SupervisedHostBridge implements HostBridge {
  private disposed = false;

  constructor(
    private readonly bridge: AuthenticatedLinuxHostBridge,
    private readonly child: HostChild,
  ) {
    const close = (): void => this.bridge.dispose();
    child.once("error", close);
    child.once("exit", close);
    child.stdout.on("data", () => this.dispose());
    child.stderr.on("data", () => this.dispose());
  }

  doctor(
    request: Parameters<HostBridge["doctor"]>[0],
  ): ReturnType<HostBridge["doctor"]> {
    return this.bridge.doctor(request);
  }

  previewDiagnosticExport(
    request: Parameters<HostBridge["previewDiagnosticExport"]>[0],
  ): ReturnType<HostBridge["previewDiagnosticExport"]> {
    return this.bridge.previewDiagnosticExport(request);
  }

  approveDiagnosticExport(
    request: Parameters<HostBridge["approveDiagnosticExport"]>[0],
  ): ReturnType<HostBridge["approveDiagnosticExport"]> {
    return this.bridge.approveDiagnosticExport(request);
  }

  cancelDiagnosticExport(
    request: Parameters<HostBridge["cancelDiagnosticExport"]>[0],
  ): ReturnType<HostBridge["cancelDiagnosticExport"]> {
    return this.bridge.cancelDiagnosticExport(request);
  }

  previewRead(
    request: Parameters<HostBridge["previewRead"]>[0],
  ): ReturnType<HostBridge["previewRead"]> {
    return this.bridge.previewRead(request);
  }

  approveRead(
    request: Parameters<HostBridge["approveRead"]>[0],
  ): ReturnType<HostBridge["approveRead"]> {
    return this.bridge.approveRead(request);
  }

  cancelRead(
    request: Parameters<HostBridge["cancelRead"]>[0],
  ): ReturnType<HostBridge["cancelRead"]> {
    return this.bridge.cancelRead(request);
  }

  dispose(): void {
    if (this.disposed) {
      return;
    }
    this.disposed = true;
    this.bridge.dispose();
    this.child.kill("SIGKILL");
  }
}

function readBootstrapFrame(
  child: HostChild,
  timeoutMs: number,
): Promise<Buffer> {
  return new Promise<Buffer>((resolve, reject) => {
    let retained = Buffer.alloc(0);
    let expected: number | undefined;
    let complete = false;
    const timer = setTimeout(() => fail(), timeoutMs);
    timer.unref();
    const cleanup = (): void => {
      clearTimeout(timer);
      child.stdout.off("data", onData);
      child.off("error", fail);
      child.off("exit", fail);
    };
    const fail = (): void => {
      if (complete) {
        return;
      }
      complete = true;
      cleanup();
      retained.fill(0);
      reject(new BootstrapFailure());
    };
    const onData = (chunk: Buffer): void => {
      if (complete || !Buffer.isBuffer(chunk)) {
        fail();
        return;
      }
      if (retained.length + chunk.length > MAX_BOOTSTRAP_BYTES) {
        fail();
        return;
      }
      retained = Buffer.concat([retained, chunk]);
      if (retained.length >= BOOTSTRAP_HEADER_BYTES && expected === undefined) {
        const endpointLength = retained.readUInt16BE(6);
        if (endpointLength === 0 || endpointLength > MAX_ENDPOINT_BYTES) {
          fail();
          return;
        }
        expected =
          BOOTSTRAP_HEADER_BYTES + endpointLength + BOOTSTRAP_FIXED_BYTES;
      }
      if (expected !== undefined && retained.length === expected) {
        complete = true;
        cleanup();
        resolve(retained);
      } else if (expected !== undefined && retained.length > expected) {
        fail();
      }
    };
    child.stdout.on("data", onData);
    child.once("error", fail);
    child.once("exit", fail);
    child.stderr.on("data", (chunk: Buffer) => {
      if (!Buffer.isBuffer(chunk) || chunk.length > MAX_BOOTSTRAP_BYTES) {
        fail();
      }
    });
  });
}

function parseBootstrapFrame(
  frame: Buffer,
  uid: number,
): LinuxHostLaunchCredentials {
  if (
    frame.length < BOOTSTRAP_HEADER_BYTES + BOOTSTRAP_FIXED_BYTES ||
    !frame.subarray(0, 4).equals(BOOTSTRAP_MAGIC) ||
    frame.readUInt16BE(4) !== BOOTSTRAP_VERSION
  ) {
    throw new BootstrapFailure();
  }
  const endpointLength = frame.readUInt16BE(6);
  const fixed = BOOTSTRAP_HEADER_BYTES + endpointLength;
  if (frame.length !== fixed + BOOTSTRAP_FIXED_BYTES) {
    throw new BootstrapFailure();
  }
  const endpointBytes = frame.subarray(BOOTSTRAP_HEADER_BYTES, fixed);
  const endpoint = endpointBytes.toString("utf8");
  const expectedPrefix = `/run/user/${uid.toString()}/agentmage/`;
  if (
    Buffer.from(endpoint, "utf8").length !== endpointBytes.length ||
    !endpoint.startsWith(expectedPrefix) ||
    endpoint.includes("\0") ||
    endpoint.includes("..") ||
    endpoint.includes("//")
  ) {
    throw new BootstrapFailure();
  }
  let offset = fixed;
  const challenge = Uint8Array.from(frame.subarray(offset, offset + 32));
  offset += 32;
  const launchSecret = Uint8Array.from(frame.subarray(offset, offset + 32));
  offset += 32;
  const observedUid = frame.readUInt32BE(offset);
  offset += 4;
  const pid = frame.readInt32BE(offset);
  offset += 4;
  const startTimeTicks = frame.readBigUInt64BE(offset);
  offset += 8;
  const executableSha256 = Uint8Array.from(frame.subarray(offset, offset + 32));
  if (observedUid !== uid) {
    launchSecret.fill(0);
    throw new BootstrapFailure();
  }
  return {
    endpoint,
    challenge,
    launchSecret,
    peer: { uid, pid, startTimeTicks, executableSha256 },
  };
}

class BootstrapFailure extends Error {
  constructor() {
    super("host.bootstrap.failed");
  }
}
