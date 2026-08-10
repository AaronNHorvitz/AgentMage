/** Stable identity for the display-only Visual Studio Code shell. */
export const COMPONENT_ID = "shell-vscode" as const;

/** Authority retained by the empty extension scaffold. */
export const SHELL_AUTHORITY = Object.freeze([
  "display",
  "interaction",
  "provider-registration",
  "authenticated-ipc-client",
] as const);
