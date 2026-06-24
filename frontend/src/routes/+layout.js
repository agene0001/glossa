// Single-page app: no server-side rendering, no prerender — everything runs in
// the Tauri webview (and a browser later) and talks to the backend via invoke().
export const ssr = false;
export const prerender = false;
