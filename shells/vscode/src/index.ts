/** Stable identity for the display-only Visual Studio Code shell. */
export const COMPONENT_ID = "shell-vscode" as const;

/** Authority retained by the display-only secure-read extension. */
export const SHELL_AUTHORITY = Object.freeze([
  "display",
  "interaction",
  "provider-registration",
  "authenticated-ipc-client",
  "request-bound-reference-resolution",
] as const);

/** Minimal registration contract used to test replacement and deactivation. */
export interface DisposableRegistration {
  dispose(): void;
}

/** Owns exactly one active provider registration. */
export class RegistrationSlot {
  private active: DisposableRegistration | undefined;

  replace(registration: DisposableRegistration): void {
    this.dispose();
    this.active = registration;
  }

  dispose(): void {
    this.active?.dispose();
    this.active = undefined;
  }
}
