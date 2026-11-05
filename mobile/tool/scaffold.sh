#!/usr/bin/env bash
# One-time platform scaffold: runs `flutter create` for ios+android into this
# directory (keeps lib/, test/, pubspec.yaml) and registers the reelearn://
# deep-link scheme plus camera/microphone permissions. Idempotent.
set -euo pipefail
cd "$(dirname "$0")/.."

if [ ! -d ios ] || [ ! -d android ]; then
  flutter create --org sa.reelearn --project-name reelearn_mobile --platforms ios,android .
fi

PLIST=ios/Runner/Info.plist
if ! grep -q "CFBundleURLSchemes" "$PLIST"; then
  /usr/libexec/PlistBuddy -c "Add :CFBundleURLTypes array" "$PLIST"
  /usr/libexec/PlistBuddy -c "Add :CFBundleURLTypes:0 dict" "$PLIST"
  /usr/libexec/PlistBuddy -c "Add :CFBundleURLTypes:0:CFBundleURLName string sa.reelearn" "$PLIST"
  /usr/libexec/PlistBuddy -c "Add :CFBundleURLTypes:0:CFBundleURLSchemes array" "$PLIST"
  /usr/libexec/PlistBuddy -c "Add :CFBundleURLTypes:0:CFBundleURLSchemes:0 string reelearn" "$PLIST"
fi
if ! grep -q "NSCameraUsageDescription" "$PLIST"; then
  /usr/libexec/PlistBuddy -c "Add :NSCameraUsageDescription string Reelearn records your 60-second reel." "$PLIST"
  /usr/libexec/PlistBuddy -c "Add :NSMicrophoneUsageDescription string Reelearn records your voice for the reel." "$PLIST"
fi
# Local HTTP API during development (localhost / LAN).
if ! grep -q "NSAllowsLocalNetworking" "$PLIST"; then
  /usr/libexec/PlistBuddy -c "Add :NSAppTransportSecurity dict" "$PLIST" 2>/dev/null || true
  /usr/libexec/PlistBuddy -c "Add :NSAppTransportSecurity:NSAllowsLocalNetworking bool true" "$PLIST"
fi

MANIFEST=android/app/src/main/AndroidManifest.xml
if ! grep -q 'android:scheme="reelearn"' "$MANIFEST"; then
  python3 - "$MANIFEST" <<'EOF'
import sys
p = sys.argv[1]
s = open(p).read()
filter_ = '''
            <intent-filter>
                <action android:name="android.intent.action.VIEW" />
                <category android:name="android.intent.category.DEFAULT" />
                <category android:name="android.intent.category.BROWSABLE" />
                <data android:scheme="reelearn" android:host="t" />
            </intent-filter>
'''
s = s.replace('</activity>', filter_ + '        </activity>', 1)
perms = '''    <uses-permission android:name="android.permission.INTERNET" />
    <uses-permission android:name="android.permission.CAMERA" />
    <uses-permission android:name="android.permission.RECORD_AUDIO" />
'''
if 'android.permission.CAMERA' not in s:
    s = s.replace('<application', perms + '    <application', 1)
if 'usesCleartextTraffic' not in s:
    s = s.replace('<application', '<application\n        android:usesCleartextTraffic="true"', 1)
open(p, 'w').write(s)
EOF
fi
echo "scaffold ok"
