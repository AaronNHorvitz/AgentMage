/** Independently removable VS Code compatibility registrations. */
export interface ExtensionFeatureActivation {
  readonly nativeParticipant: boolean;
  readonly nativeProviderCompatibility: boolean;
}

/** Resolves only exact booleans; malformed values fail disabled. */
export function resolveExtensionFeatureActivation(
  read: (key: string) => unknown,
): ExtensionFeatureActivation {
  return {
    nativeParticipant: read("nativeParticipant") === true,
    nativeProviderCompatibility: read("nativeProviderCompatibility") === true,
  };
}
