#!/usr/bin/env bash
# Creates the Android upload key (once) and stores it as GitHub secrets so the
# Release workflow can sign APKs.
#
# The key is kept OUTSIDE the repository, in ~/.ecosystem-connector-release/.
# Back that folder up somewhere safe: without the same key, already installed
# copies of the app cannot be updated and must be uninstalled first.
#
# Usage: tools/android/setup-release-signing.sh [owner/repo]
set -euo pipefail

repo="${1:-SerhanEnsar/ecosystem-connector}"
key_dir="${EC_RELEASE_KEY_DIR:-$HOME/.ecosystem-connector-release}"
keystore="$key_dir/upload-keystore.p12"
password_file="$key_dir/keystore-password.txt"
alias_name="release"

command -v gh >/dev/null || { echo "GitHub CLI (gh) is required." >&2; exit 1; }

keytool_bin="$(command -v keytool || true)"
if [[ -z "$keytool_bin" && -x "/Applications/Android Studio.app/Contents/jbr/Contents/Home/bin/keytool" ]]; then
  keytool_bin="/Applications/Android Studio.app/Contents/jbr/Contents/Home/bin/keytool"
fi

mkdir -p "$key_dir"
chmod 700 "$key_dir"

if [[ ! -f "$keystore" ]]; then
  [[ -n "$keytool_bin" ]] || { echo "keytool not found (install a JDK or Android Studio)." >&2; exit 1; }
  echo "Creating a new upload key in $key_dir"
  (umask 077; openssl rand -base64 48 | tr -d '/+=\n' | head -c 32 > "$password_file")
  password="$(cat "$password_file")"
  "$keytool_bin" -genkeypair -keystore "$keystore" -storetype PKCS12 -alias "$alias_name" \
    -keyalg RSA -keysize 4096 -validity 10000 \
    -dname "CN=SerhanEnsar, O=ECOnnector" \
    -storepass "$password" -keypass "$password" >/dev/null
  chmod 600 "$keystore"
else
  echo "Using the existing upload key in $key_dir"
fi

echo "Creating the 'release' environment (only tags v* may use it)"
gh api -X PUT "repos/$repo/environments/release" --input - >/dev/null <<'JSON'
{"deployment_branch_policy":{"protected_branches":false,"custom_branch_policies":true}}
JSON
gh api -X POST "repos/$repo/environments/release/deployment-branch-policies" \
  -f name='v*' -f type=tag >/dev/null 2>&1 || true  # already exists

echo "Uploading secrets to the 'release' environment"
base64 < "$keystore" | tr -d '\n' | gh secret set ANDROID_KEYSTORE_BASE64 --env release -R "$repo"
gh secret set ANDROID_KEYSTORE_PASSWORD --env release -R "$repo" < "$password_file"
gh secret set ANDROID_KEY_PASSWORD --env release -R "$repo" < "$password_file"
printf '%s' "$alias_name" | gh secret set ANDROID_KEY_ALIAS --env release -R "$repo"

echo
echo "Done. Signing certificate fingerprint (SHA-256):"
"$keytool_bin" -list -keystore "$keystore" -storepass "$(cat "$password_file")" -alias "$alias_name" \
  | grep -i "SHA-256\|SHA256" || true
echo
echo "Remember to back up $key_dir."
