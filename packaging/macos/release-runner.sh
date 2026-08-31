#!/bin/bash
# Manual AgentMage macOS release ceremony. Never source this file.
set -euo pipefail
IFS=$'\n\t'
umask 077

readonly FIXED_NOTARY_PROFILE="agentmage-release-notary-v1"
readonly FIXED_TOOLCHAIN_ROOT="/opt/agentmage-release-toolchain/bin"
readonly FIXED_CARGO="$FIXED_TOOLCHAIN_ROOT/cargo"
readonly FIXED_NPM="$FIXED_TOOLCHAIN_ROOT/npm"
readonly FIXED_XCODE_PROJECT="platforms/macos/AgentMage.xcodeproj"
readonly FIXED_XCODE_SCHEME="AgentMageRelease"
readonly FIXED_INSTALL_RELATIVE="Applications/AgentMage.app"
readonly FIXED_APP_EXECUTABLE="Contents/MacOS/AgentMageKernelHost"
readonly FIXED_BRIDGE_EXECUTABLE="Contents/MacOS/AgentMageVSCodeBridge"
readonly FIXED_TOOL_XPC="Contents/XPCServices/AgentMageXPCToolHelper.xpc"
readonly FIXED_INFERENCE_XPC="Contents/XPCServices/AgentMageMetalInferenceService.xpc"

fail() {
  /bin/echo "macos.release-runner.$1" >&2
  exit 1
}

[[ $# -eq 2 ]] || fail "usage"
readonly POLICY_INPUT="$1"
readonly EVIDENCE_INPUT="$2"
[[ "$POLICY_INPUT" = /* && "$EVIDENCE_INPUT" = /* ]] || fail "absolute-path-required"
[[ -f "$POLICY_INPUT" && ! -L "$POLICY_INPUT" ]] || fail "policy-unavailable"
[[ ! -e "$EVIDENCE_INPUT" ]] || fail "evidence-destination-not-new"

readonly SOURCE_ROOT="$(/usr/bin/git rev-parse --show-toplevel 2>/dev/null)" || fail "source-root"
cd "$SOURCE_ROOT"
case "$EVIDENCE_INPUT" in
  "$SOURCE_ROOT"|"$SOURCE_ROOT"/*) fail "evidence-inside-source" ;;
esac

policy_raw() {
  /usr/bin/plutil -extract "$1" raw -o - "$POLICY_INPUT" 2>/dev/null || fail "policy-field"
}

readonly POLICY_STATUS="$(policy_raw status)"
readonly SOURCE_REVISION="$(policy_raw source_revision)"
readonly VERSION="$(policy_raw version)"
readonly EXPECTED_MACOS_BUILD="$(policy_raw expected_macos_build)"
readonly EXPECTED_XCODE_BUILD="$(policy_raw expected_xcode_build)"
readonly ARCHITECTURE="$(policy_raw architecture)"
readonly TEAM_ID="$(policy_raw team_id)"
readonly APPLICATION_IDENTITY="$(policy_raw application_identity_sha1)"
readonly INSTALLER_IDENTITY="$(policy_raw installer_identity_sha1)"
readonly PREVIOUS_PACKAGE="$(policy_raw previous_package_path)"
readonly PREVIOUS_PACKAGE_SHA256="$(policy_raw previous_package_sha256)"
readonly NOTARY_PROFILE="$(policy_raw notary_keychain_profile)"
readonly CREDENTIAL_VALUES_PRESENT="$(policy_raw credential_values_present)"
readonly PRIVATE_ENVIRONMENT_VALUES_PRESENT="$(policy_raw private_environment_values_present)"
readonly RELEASE_CLAIM="$(policy_raw release_claim)"

[[ "$POLICY_STATUS" = "release-approved" ]] || fail "policy-not-release-approved"
[[ "$RELEASE_CLAIM" = "authorized-candidate" ]] || fail "policy-release-claim"
readonly EXPECTED_POLICY_KEYS=$'app_group_identifier\napplication_identity_sha1\narchitecture\nbundle_identifiers\nceremony_id\ncredential_values_present\nentitlements\nexpected_macos_build\nexpected_xcode_build\ninstaller_identity_sha1\nkeychain_access_group\nnotary_keychain_profile\nprevious_package_path\nprevious_package_sha256\nprivate_environment_values_present\nrecord_type\nrelease_claim\nschema_version\nsource_revision\nstatus\nteam_id\nversion'
readonly OBSERVED_POLICY_KEYS="$(/usr/bin/plutil -p "$POLICY_INPUT" | /usr/bin/sed -n 's/^  "\([^"]*\)" =>.*/\1/p' | LC_ALL=C /usr/bin/sort)"
[[ "$OBSERVED_POLICY_KEYS" = "$EXPECTED_POLICY_KEYS" ]] || fail "policy-field-closure"
[[ "$(/usr/bin/stat -f '%u:%Lp:%l' "$POLICY_INPUT")" = "$(/usr/bin/id -u):600:1" ]] || fail "policy-file-metadata"
[[ "$CREDENTIAL_VALUES_PRESENT" = "false" ]] || fail "credential-value-in-policy"
[[ "$PRIVATE_ENVIRONMENT_VALUES_PRESENT" = "false" ]] || fail "private-environment-in-policy"
[[ "$NOTARY_PROFILE" = "$FIXED_NOTARY_PROFILE" ]] || fail "notary-profile"
[[ "$SOURCE_REVISION" =~ ^[0-9a-f]{40}$ ]] || fail "source-revision"
[[ "$VERSION" =~ ^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$ ]] || fail "version"
[[ "$TEAM_ID" =~ ^[A-Z0-9]{10}$ ]] || fail "team-id"
[[ "$APPLICATION_IDENTITY" =~ ^[0-9A-Fa-f]{40}$ ]] || fail "application-identity"
[[ "$INSTALLER_IDENTITY" =~ ^[0-9A-Fa-f]{40}$ ]] || fail "installer-identity"
[[ "$APPLICATION_IDENTITY" != "$INSTALLER_IDENTITY" ]] || fail "identity-class-collision"
readonly KERNEL_BUNDLE="$(policy_raw bundle_identifiers.kernel_host)"
readonly BRIDGE_BUNDLE="$(policy_raw bundle_identifiers.vscode_bridge)"
readonly TOOL_BUNDLE="$(policy_raw bundle_identifiers.xpc_tool_helper)"
readonly INFERENCE_BUNDLE="$(policy_raw bundle_identifiers.metal_inference_service)"
for bundle in "$KERNEL_BUNDLE" "$BRIDGE_BUNDLE" "$TOOL_BUNDLE" "$INFERENCE_BUNDLE"; do
  [[ "$bundle" =~ ^[A-Za-z0-9][A-Za-z0-9.-]{7,191}$ ]] || fail "bundle-identifier"
done
[[ "$KERNEL_BUNDLE" != "$BRIDGE_BUNDLE" && "$KERNEL_BUNDLE" != "$TOOL_BUNDLE" && "$KERNEL_BUNDLE" != "$INFERENCE_BUNDLE" && "$BRIDGE_BUNDLE" != "$TOOL_BUNDLE" && "$BRIDGE_BUNDLE" != "$INFERENCE_BUNDLE" && "$TOOL_BUNDLE" != "$INFERENCE_BUNDLE" ]] || fail "bundle-identity-collision"
[[ "$(policy_raw keychain_access_group)" = "$TEAM_ID."* ]] || fail "keychain-team-binding"
validate_policy_entitlements() {
  local component="$1"
  local expected_count="$2"
  shift 2
  local observed
  observed="$(/usr/bin/plutil -extract "entitlements.$component" xml1 -o - "$POLICY_INPUT")" || fail "policy-entitlement-closure"
  [[ "$(/usr/bin/printf '%s' "$observed" | /usr/bin/grep -o '<string>' | /usr/bin/wc -l | /usr/bin/tr -d ' ')" = "$expected_count" ]] || fail "policy-entitlement-closure"
  for entitlement in "$@"; do
    /usr/bin/printf '%s' "$observed" | /usr/bin/grep -F "<string>$entitlement</string>" >/dev/null || fail "policy-entitlement-closure"
  done
}
validate_policy_entitlements kernel_host 5 com.apple.security.app-sandbox com.apple.security.application-groups com.apple.security.files.bookmarks.app-scope com.apple.security.files.user-selected.read-only keychain-access-groups
validate_policy_entitlements vscode_bridge 2 com.apple.security.app-sandbox com.apple.security.application-groups
validate_policy_entitlements xpc_tool_helper 2 com.apple.security.app-sandbox com.apple.security.files.bookmarks.app-scope
validate_policy_entitlements metal_inference_service 1 com.apple.security.app-sandbox
[[ "$PREVIOUS_PACKAGE" = /* && -f "$PREVIOUS_PACKAGE" && ! -L "$PREVIOUS_PACKAGE" ]] || fail "previous-package"

[[ "$(/usr/bin/uname -s)" = "Darwin" ]] || fail "not-macos"
[[ "$(/usr/bin/uname -m)" = "arm64" && "$ARCHITECTURE" = "arm64" ]] || fail "not-arm64"
[[ "$(/usr/bin/id -u)" != "0" ]] || fail "root-forbidden"
[[ "$(/usr/bin/sw_vers -buildVersion)" = "$EXPECTED_MACOS_BUILD" ]] || fail "macos-build"
readonly OBSERVED_XCODE_BUILD="$(/usr/bin/xcodebuild -version | /usr/bin/awk '/Build version/{print $3}')"
[[ "$OBSERVED_XCODE_BUILD" = "$EXPECTED_XCODE_BUILD" ]] || fail "xcode-build"
[[ "$(/usr/bin/git rev-parse --verify HEAD)" = "$SOURCE_REVISION" ]] || fail "wrong-source-revision"
! /usr/bin/git symbolic-ref -q HEAD >/dev/null 2>&1 || fail "source-not-detached"
[[ -z "$(/usr/bin/git status --porcelain=v1 --untracked-files=all)" ]] || fail "dirty-source"
[[ -f "$FIXED_XCODE_PROJECT/project.pbxproj" ]] || fail "release-project-unavailable"
[[ -f "$FIXED_XCODE_PROJECT/xcshareddata/xcschemes/$FIXED_XCODE_SCHEME.xcscheme" ]] || fail "release-scheme-unavailable"
[[ -x "$FIXED_CARGO" && ! -L "$FIXED_CARGO" ]] || fail "release-cargo-unavailable"
[[ -x "$FIXED_NPM" && ! -L "$FIXED_NPM" ]] || fail "release-npm-unavailable"

readonly PREVIOUS_OBSERVED_SHA256="$(/usr/bin/shasum -a 256 "$PREVIOUS_PACKAGE" | /usr/bin/awk '{print $1}')"
[[ "$PREVIOUS_OBSERVED_SHA256" = "$PREVIOUS_PACKAGE_SHA256" ]] || fail "previous-package-digest"

readonly INSTALLED_APP="$HOME/$FIXED_INSTALL_RELATIVE"
[[ "$INSTALLED_APP" = "$HOME/Applications/AgentMage.app" ]] || fail "install-path"
[[ ! -e "$INSTALLED_APP" && ! -L "$INSTALLED_APP" ]] || fail "preexisting-install"

/bin/mkdir -m 700 "$EVIDENCE_INPUT" || fail "evidence-create"
readonly EVIDENCE_ROOT="$(cd "$EVIDENCE_INPUT" && /bin/pwd -P)"
case "$EVIDENCE_ROOT" in
  "$SOURCE_ROOT"|"$SOURCE_ROOT"/*) fail "resolved-evidence-inside-source" ;;
esac
readonly WORK_ROOT="$(/usr/bin/mktemp -d "${TMPDIR:-/tmp}/agentmage-macos-release.XXXXXXXX")"
case "$WORK_ROOT" in
  "${TMPDIR:-/tmp}"/agentmage-macos-release.*) ;;
  *) fail "private-staging-path" ;;
esac

cleanup() {
  case "$WORK_ROOT" in
    "${TMPDIR:-/tmp}"/agentmage-macos-release.*) /bin/rm -rf -- "$WORK_ROOT" ;;
  esac
}
trap cleanup EXIT HUP INT TERM

readonly LOG_ROOT="$EVIDENCE_ROOT/command-logs"
/bin/mkdir -m 700 "$LOG_ROOT"
COMMAND_SEQUENCE=0
run_step() {
  local step="$1"
  shift
  COMMAND_SEQUENCE=$((COMMAND_SEQUENCE + 1))
  local log="$LOG_ROOT/$(/usr/bin/printf '%03d-%s.log' "$COMMAND_SEQUENCE" "$step")"
  env -i HOME="$HOME" PATH="$FIXED_TOOLCHAIN_ROOT:/usr/bin:/bin:/usr/sbin:/sbin" TMPDIR="$WORK_ROOT" \
    "$@" >"$log" 2>&1 || fail "$step"
}

# Certificate queries contain public identity metadata only. Private key and
# notary credential values are never exported or accepted as arguments. The
# complete Keychain identity listing remains only in private staging.
/usr/bin/security find-identity -v -p codesigning >"$WORK_ROOT/application-identities" 2>&1 || fail "application-identity-query"
/usr/bin/security find-identity -v -p basic >"$WORK_ROOT/installer-identities" 2>&1 || fail "installer-identity-query"
[[ "$(/usr/bin/grep -Fic "$APPLICATION_IDENTITY" "$WORK_ROOT/application-identities")" = "1" ]] || fail "application-identity-not-exact"
[[ "$(/usr/bin/grep -Fic "$INSTALLER_IDENTITY" "$WORK_ROOT/installer-identities")" = "1" ]] || fail "installer-identity-not-exact"
/bin/rm -f -- "$WORK_ROOT/application-identities" "$WORK_ROOT/installer-identities"

run_step "rust-build" "$FIXED_CARGO" build --workspace --all-targets --release --locked --target aarch64-apple-darwin
run_step "typescript-build" "$FIXED_NPM" run build --workspace @agentmage/vscode-shell
run_step "swift-test" /usr/bin/swift test --package-path platforms/macos

readonly ARCHIVE_PATH="$WORK_ROOT/AgentMage.xcarchive"
run_step "xcode-archive" /usr/bin/xcodebuild \
  -project "$FIXED_XCODE_PROJECT" \
  -scheme "$FIXED_XCODE_SCHEME" \
  -configuration Release \
  -destination "generic/platform=macOS,arch=arm64" \
  -archivePath "$ARCHIVE_PATH" \
  ARCHS=arm64 ONLY_ACTIVE_ARCH=YES \
  DEVELOPMENT_TEAM="$TEAM_ID" \
  CODE_SIGN_STYLE=Manual \
  CODE_SIGN_IDENTITY="$APPLICATION_IDENTITY" \
  OTHER_CODE_SIGN_FLAGS="--timestamp --options runtime" \
  archive

readonly APP_PATH="$ARCHIVE_PATH/Products/Applications/AgentMage.app"
[[ -d "$APP_PATH" && ! -L "$APP_PATH" ]] || fail "archived-app-unavailable"
for relative in "$FIXED_APP_EXECUTABLE" "$FIXED_BRIDGE_EXECUTABLE" "$FIXED_TOOL_XPC" "$FIXED_INFERENCE_XPC"; do
  [[ -e "$APP_PATH/$relative" && ! -L "$APP_PATH/$relative" ]] || fail "component-closure"
done

run_step "codesign-deep-verify" /usr/bin/codesign --verify --deep --strict --verbose=4 "$APP_PATH"
verify_component() {
  local index="$1"
  local policy_name="$2"
  local relative="$3"
  local expected_key_count="$4"
  local expected_identifier
  expected_identifier="$(policy_raw "bundle_identifiers.$policy_name")"
  run_step "component-${index}-identity" /usr/bin/codesign -d --verbose=4 --requirements :- "$APP_PATH/$relative"
  local identity_log="$LOG_ROOT/$(/usr/bin/printf '%03d-component-%s-identity.log' "$COMMAND_SEQUENCE" "$index")"
  /usr/bin/grep -F "TeamIdentifier=$TEAM_ID" "$identity_log" >/dev/null || fail "component-team"
  /usr/bin/grep -F "Identifier=$expected_identifier" "$identity_log" >/dev/null || fail "component-identifier"
  /usr/bin/grep -F "flags=" "$identity_log" | /usr/bin/grep -F "runtime" >/dev/null || fail "hardened-runtime"
  COMMAND_SEQUENCE=$((COMMAND_SEQUENCE + 1))
  local entitlement_plist="$LOG_ROOT/$(/usr/bin/printf '%03d-component-%s-entitlements.plist' "$COMMAND_SEQUENCE" "$index")"
  local entitlement_stderr="$LOG_ROOT/$(/usr/bin/printf '%03d-component-%s-entitlements.stderr' "$COMMAND_SEQUENCE" "$index")"
  env -i HOME="$HOME" PATH="/usr/bin:/bin:/usr/sbin:/sbin" TMPDIR="$WORK_ROOT" \
    /usr/bin/codesign -d --entitlements :- "$APP_PATH/$relative" >"$entitlement_plist" 2>"$entitlement_stderr" || fail "component-entitlements"
  [[ "$(/usr/bin/grep -o '<key>' "$entitlement_plist" | /usr/bin/wc -l | /usr/bin/tr -d ' ')" = "$expected_key_count" ]] || fail "component-entitlement-closure"
}

verify_component 1 kernel_host "$FIXED_APP_EXECUTABLE" 5
verify_component 2 vscode_bridge "$FIXED_BRIDGE_EXECUTABLE" 2
verify_component 3 xpc_tool_helper "$FIXED_TOOL_XPC" 2
verify_component 4 metal_inference_service "$FIXED_INFERENCE_XPC" 1
! /usr/bin/grep -R -F "com.apple.security.get-task-allow" "$LOG_ROOT" >/dev/null || fail "get-task-allow"
! /usr/bin/grep -R -E 'com\.apple\.security\.(network\.(client|server)|cs\.(allow-jit|disable-library-validation))' "$LOG_ROOT" >/dev/null || fail "prohibited-entitlement"
for required_key in com.apple.security.app-sandbox; do
  [[ "$(/usr/bin/grep -R -l -F "<key>$required_key</key>" "$LOG_ROOT" | /usr/bin/wc -l | /usr/bin/tr -d ' ')" = "4" ]] || fail "sandbox-entitlement-closure"
done
[[ "$(/usr/bin/grep -R -l -F '<key>com.apple.security.application-groups</key>' "$LOG_ROOT" | /usr/bin/wc -l | /usr/bin/tr -d ' ')" = "2" ]] || fail "app-group-entitlement-closure"
[[ "$(/usr/bin/grep -R -l -F '<key>com.apple.security.files.bookmarks.app-scope</key>' "$LOG_ROOT" | /usr/bin/wc -l | /usr/bin/tr -d ' ')" = "2" ]] || fail "bookmark-entitlement-closure"
[[ "$(/usr/bin/grep -R -l -F '<key>com.apple.security.files.user-selected.read-only</key>' "$LOG_ROOT" | /usr/bin/wc -l | /usr/bin/tr -d ' ')" = "1" ]] || fail "read-only-entitlement-closure"
[[ "$(/usr/bin/grep -R -l -F '<key>keychain-access-groups</key>' "$LOG_ROOT" | /usr/bin/wc -l | /usr/bin/tr -d ' ')" = "1" ]] || fail "keychain-entitlement-closure"
[[ "$(/usr/bin/grep -R -l -F "<string>$(policy_raw app_group_identifier)</string>" "$LOG_ROOT" | /usr/bin/wc -l | /usr/bin/tr -d ' ')" = "2" ]] || fail "app-group-value"
[[ "$(/usr/bin/grep -R -l -F "<string>$(policy_raw keychain_access_group)</string>" "$LOG_ROOT" | /usr/bin/wc -l | /usr/bin/tr -d ' ')" = "1" ]] || fail "keychain-group-value"

readonly PAYLOAD_ROOT="$WORK_ROOT/payload"
/bin/mkdir -p "$PAYLOAD_ROOT/Applications"
/usr/bin/ditto "$APP_PATH" "$PAYLOAD_ROOT/Applications/AgentMage.app"
readonly UNSIGNED_PACKAGE="$WORK_ROOT/AgentMage-unsigned.pkg"
readonly SIGNED_PACKAGE="$WORK_ROOT/AgentMage-$VERSION.pkg"
run_step "pkgbuild" /usr/bin/pkgbuild --root "$PAYLOAD_ROOT" --install-location / --identifier "$(policy_raw bundle_identifiers.kernel_host).installer" --version "$VERSION" "$UNSIGNED_PACKAGE"
run_step "productsign" /usr/bin/productsign --sign "$INSTALLER_IDENTITY" "$UNSIGNED_PACKAGE" "$SIGNED_PACKAGE"
run_step "package-signature" /usr/sbin/pkgutil --check-signature "$SIGNED_PACKAGE"

readonly NOTARY_RESULT="$EVIDENCE_ROOT/notary-submit.json"
env -i HOME="$HOME" PATH="/usr/bin:/bin:/usr/sbin:/sbin" TMPDIR="$WORK_ROOT" \
  /usr/bin/xcrun notarytool submit "$SIGNED_PACKAGE" --keychain-profile "$FIXED_NOTARY_PROFILE" --wait --output-format json \
  >"$NOTARY_RESULT" 2>"$LOG_ROOT/notary-submit.stderr" || fail "notary-submit"
[[ "$(/usr/bin/plutil -extract status raw -o - "$NOTARY_RESULT")" = "Accepted" ]] || fail "notary-not-accepted"
readonly NOTARY_ID="$(/usr/bin/plutil -extract id raw -o - "$NOTARY_RESULT")"
[[ "$NOTARY_ID" =~ ^[0-9A-Fa-f-]{36}$ ]] || fail "notary-id"
run_step "notary-log" /usr/bin/xcrun notarytool log "$NOTARY_ID" --keychain-profile "$FIXED_NOTARY_PROFILE" "$EVIDENCE_ROOT/notary-log.json"
[[ "$(/usr/bin/plutil -extract status raw -o - "$EVIDENCE_ROOT/notary-log.json")" = "Accepted" ]] || fail "notary-log-status"
! /usr/bin/grep -E '"severity"[[:space:]]*:[[:space:]]*"(warning|error)"' "$EVIDENCE_ROOT/notary-log.json" >/dev/null || fail "notary-log-issues"
run_step "staple" /usr/bin/xcrun stapler staple "$SIGNED_PACKAGE"
run_step "staple-validate" /usr/bin/xcrun stapler validate "$SIGNED_PACKAGE"
run_step "gatekeeper-install" /usr/sbin/spctl --assess --type install --verbose=4 "$SIGNED_PACKAGE"

# The runner's external network control must be closed before this point.
run_step "candidate-install" /usr/sbin/installer -pkg "$SIGNED_PACKAGE" -target CurrentUserHomeDirectory
[[ -d "$INSTALLED_APP" && ! -L "$INSTALLED_APP" ]] || fail "candidate-install-postcondition"
run_step "candidate-launch" "$INSTALLED_APP/$FIXED_APP_EXECUTABLE" --release-smoke-test --content-free-output "$EVIDENCE_ROOT/candidate-smoke.json"
run_step "candidate-cleanup" "$INSTALLED_APP/$FIXED_APP_EXECUTABLE" --release-uninstall-smoke --content-free-output "$EVIDENCE_ROOT/candidate-uninstall.json"

case "$INSTALLED_APP" in
  "$HOME/Applications/AgentMage.app") /bin/rm -rf -- "$INSTALLED_APP" ;;
  *) fail "uninstall-path" ;;
esac
[[ ! -e "$INSTALLED_APP" && ! -L "$INSTALLED_APP" ]] || fail "uninstall-postcondition"
run_step "candidate-forget-receipt" /usr/sbin/pkgutil --forget "$(policy_raw bundle_identifiers.kernel_host).installer"
for residue in "$HOME/Library/Application Support/AgentMage" "$HOME/Library/Caches/AgentMage" "$HOME/Library/Preferences/$(policy_raw bundle_identifiers.kernel_host).plist"; do
  [[ ! -e "$residue" && ! -L "$residue" ]] || fail "uninstall-residue"
done

run_step "candidate-reinstall" /usr/sbin/installer -pkg "$SIGNED_PACKAGE" -target CurrentUserHomeDirectory
[[ -d "$INSTALLED_APP" && ! -L "$INSTALLED_APP" ]] || fail "candidate-reinstall-postcondition"
run_step "candidate-rollback-cleanup" "$INSTALLED_APP/$FIXED_APP_EXECUTABLE" --release-uninstall-smoke --content-free-output "$EVIDENCE_ROOT/candidate-rollback-cleanup.json"
case "$INSTALLED_APP" in
  "$HOME/Applications/AgentMage.app") /bin/rm -rf -- "$INSTALLED_APP" ;;
  *) fail "rollback-remove-path" ;;
esac
run_step "candidate-rollback-forget-receipt" /usr/sbin/pkgutil --forget "$(policy_raw bundle_identifiers.kernel_host).installer"
run_step "previous-package-signature" /usr/sbin/pkgutil --check-signature "$PREVIOUS_PACKAGE"
run_step "previous-package-staple" /usr/bin/xcrun stapler validate "$PREVIOUS_PACKAGE"
run_step "previous-package-gatekeeper" /usr/sbin/spctl --assess --type install --verbose=4 "$PREVIOUS_PACKAGE"
run_step "rollback-install" /usr/sbin/installer -pkg "$PREVIOUS_PACKAGE" -target CurrentUserHomeDirectory
[[ -d "$INSTALLED_APP" && ! -L "$INSTALLED_APP" ]] || fail "rollback-install-postcondition"
run_step "rollback-launch" "$INSTALLED_APP/$FIXED_APP_EXECUTABLE" --release-smoke-test --content-free-output "$EVIDENCE_ROOT/rollback-smoke.json"

readonly FINAL_PACKAGE="$EVIDENCE_ROOT/AgentMage-$VERSION.pkg"
/usr/bin/ditto "$SIGNED_PACKAGE" "$FINAL_PACKAGE"
readonly FINAL_PACKAGE_SHA256="$(/usr/bin/shasum -a 256 "$FINAL_PACKAGE" | /usr/bin/awk '{print $1}')"

! /usr/bin/grep -R -E -i '(BEGIN (RSA |EC |OPENSSH )?PRIVATE KEY|apple[-_ ]?id|api[-_ ]?key|issuer[-_ ]?id|password|authorization:[[:space:]]*bearer)' "$EVIDENCE_ROOT" >/dev/null || fail "credential-shaped-evidence"

readonly TERMINAL_TMP="$WORK_ROOT/terminal.json"
/usr/bin/printf '{"schema_version":1,"record_type":"macos-release-runner-terminal","source_revision":"%s","version":"%s","macos_build":"%s","xcode_build":"%s","architecture":"arm64","package_sha256":"%s","notary_status":"Accepted","staple_valid":true,"gatekeeper_install_accepted":true,"candidate_install":true,"candidate_launch":true,"candidate_uninstall":true,"rollback_install":true,"rollback_launch":true,"credential_values_present":false,"release_claim":"signed-package-candidate"}\n' \
  "$SOURCE_REVISION" "$VERSION" "$EXPECTED_MACOS_BUILD" "$EXPECTED_XCODE_BUILD" "$FINAL_PACKAGE_SHA256" >"$TERMINAL_TMP"
/bin/mv "$TERMINAL_TMP" "$EVIDENCE_ROOT/terminal.json"
/bin/echo "macos.release-runner.signed-package-candidate"
