#!/bin/bash
# Creates a free, self-signed code signing certificate for the Mac build.
#
# Why: without a fixed signature every release has a new identity, and
# macOS forgets Full Disk Access and Automation permissions after each
# update. Signed with the same certificate, every version is "the same app"
# and permissions stay. (It does not replace an Apple Developer ID: the
# first install still needs "Open Anyway" in System Settings.)
#
# Run once, on any Mac:   bash scripts/mac-signing-cert.sh
# Then add the three printed values as repository secrets (see
# docs/RELEASES.md). Keep trufi-signing/ private; do not commit it.

set -euo pipefail

NAME="Trufi Cleaner Self-Signed"
DIR="trufi-signing"
# macOS's own LibreSSL writes .p12 files the macOS keychain can import;
# OpenSSL 3 from Homebrew needs -legacy for that.
OPENSSL=/usr/bin/openssl
[ -x "$OPENSSL" ] || OPENSSL=openssl
LEGACY=""
if "$OPENSSL" version | grep -q '^OpenSSL 3'; then LEGACY="-legacy"; fi

if [ -e "$DIR" ]; then
  echo "$DIR already exists. Reuse it: a new certificate would reset permissions once more." >&2
  exit 1
fi
mkdir -m 700 "$DIR"
cd "$DIR"

cat > cert.cnf <<EOF
[req]
distinguished_name = dn
prompt = no
x509_extensions = ext
[dn]
CN = $NAME
[ext]
basicConstraints = critical, CA:false
keyUsage = critical, digitalSignature
extendedKeyUsage = critical, codeSigning
EOF

"$OPENSSL" req -x509 -newkey rsa:2048 -nodes -keyout key.pem -out cert.pem -days 3650 -config cert.cnf 2>/dev/null
PASSWORD=$("$OPENSSL" rand -hex 16)
"$OPENSSL" pkcs12 -export $LEGACY -inkey key.pem -in cert.pem -name "$NAME" -out cert.p12 -passout "pass:$PASSWORD"
base64 < cert.p12 | tr -d '\n' > cert.p12.base64
printf '%s' "$PASSWORD" > password.txt
rm -f key.pem cert.cnf

cat <<EOF

Done. Add these repository secrets
(GitHub -> Settings -> Secrets and variables -> Actions -> New repository secret):

  APPLE_CERTIFICATE           contents of $DIR/cert.p12.base64
  APPLE_CERTIFICATE_PASSWORD  contents of $DIR/password.txt
  APPLE_SIGNING_IDENTITY      $NAME

To copy a value on a Mac:  pbcopy < $DIR/cert.p12.base64
Keep the $DIR folder somewhere safe and private.
EOF
