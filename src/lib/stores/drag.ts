// Module-level drag state — bypasses getData() unreliability on WebKit (macOS/Tauri).
// WKWebView returns empty string for custom MIME types in the drop event even though
// types.includes() works in dragover. Reading from this object is always reliable.
export const activeDrag: { requestId: string; collectionId: string } = {
  requestId: '',
  collectionId: '',
};
