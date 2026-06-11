#!/bin/bash
# Register Synape:// URL scheme for development mode
# Run this once after bun tauri dev starts

APP_NAME="Synapse"
BINARY_PATH="$HOME/Volumes/working/Synape/src-tauri/target/debug/app"

if [ ! -f "$BINARY_PATH" ]; then
  echo "Dev binary not found at $BINARY_PATH"
  echo "Make sure bun tauri dev is running first."
  exit 1
fi

# Register the scheme via macOS default handler
/System/Library/Frameworks/CoreServices.framework/Versions/A/Frameworks/LaunchServices.framework/Versions/A/Support/lsregister -f "$BINARY_PATH"

echo "Registered $APP_NAME for Synape:// scheme"
echo "Try: open 'Synape://oauth-callback?provider=github&code=test'"
