#!/bin/bash
# CI only (throwaway macOS runner): makes a self-signed code signing
# certificate usable by codesign. Imports the .p12 into a temporary
# keychain on the search list and trusts the certificate for code signing.
#
# Tauri, given a certificate file, only accepts Apple-issued names
# ("Developer ID Application: ..."), so for a self-signed one the release
# passes just APPLE_SIGNING_IDENTITY and lets codesign find it here.
#
#   scripts/ci-mac-keychain.sh <cert.p12> <p12 password>

set -euo pipefail
P12="$1"
PASSWORD="$2"
TMP="${RUNNER_TEMP:-/tmp}"
KC="$TMP/trufi-signing.keychain-db"
KC_PASSWORD=$(/usr/bin/openssl rand -hex 16)

security create-keychain -p "$KC_PASSWORD" "$KC"
security set-keychain-settings -lut 21600 "$KC"
security unlock-keychain -p "$KC_PASSWORD" "$KC"
security import "$P12" -k "$KC" -P "$PASSWORD" -T /usr/bin/codesign
security set-key-partition-list -S apple-tool:,apple:,codesign: -s -k "$KC_PASSWORD" "$KC" > /dev/null
# Keep the existing keychains searchable too.
# shellcheck disable=SC2046
security list-keychains -d user -s "$KC" $(security list-keychains -d user | tr -d '"')

/usr/bin/openssl pkcs12 -in "$P12" -nokeys -passin "pass:$PASSWORD" -out "$TMP/trufi-cert.pem"
sudo security add-trusted-cert -d -r trustRoot -p codeSign -k /Library/Keychains/System.keychain "$TMP/trufi-cert.pem"
rm -f "$TMP/trufi-cert.pem"

security find-identity -v -p codesigning "$KC"
