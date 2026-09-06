//! The manager's stylesheet.
//!
//! Inlined rather than shipped as an asset so that a storybook is a single wasm
//! bundle with no extra fetch, and so the manager's CSS is trivially separable
//! from the user's when the preview moves into an iframe at M3.

/// The manager shell stylesheet.
pub const MANAGER_CSS: &str = r#"
*{box-sizing:border-box}
body{margin:0;font:14px/1.5 ui-sans-serif,system-ui,-apple-system,sans-serif;color:#17191C;background:#FAFAF8}
.dxsb{display:grid;grid-template-columns:264px 1fr;height:100vh;outline:none}

.dxsb-sidebar{border-right:1px solid #E2E1DB;background:#F2F2EF;display:flex;flex-direction:column;min-height:0}
.dxsb-brand{font:600 13px ui-monospace,SFMono-Regular,monospace;letter-spacing:.04em;padding:14px 14px 10px;display:flex;align-items:center;gap:7px}
.dxsb-badge{background:#C4451B;color:#fff;border-radius:3px;padding:1px 5px;font-size:10px;font-weight:600}
.dxsb-searchwrap{padding:0 14px 10px}
.dxsb-search{width:100%;padding:6px 9px;border:1px solid #CFCEC7;border-radius:6px;font:inherit;background:#fff}
.dxsb-search:focus{outline:2px solid #C4451B;outline-offset:-1px;border-color:#C4451B}
.dxsb-tree{flex:1;overflow-y:auto;padding:0 8px 14px}
.dxsb-row{display:flex;align-items:center;gap:6px;width:100%;text-align:left;background:none;border:0;border-radius:5px;padding:5px 8px;font:inherit;color:#4A5058;cursor:pointer}
.dxsb-row:hover{background:#E7E6E1}
.dxsb-row.cursor{box-shadow:inset 0 0 0 1.5px #C4451B}
.dxsb-row.selected{background:#C4451B;color:#fff;font-weight:500}
.dxsb-row.selected:hover{background:#B03D18}
.dxsb-group{font-weight:600;color:#17191C}
.dxsb-caret{display:inline-block;width:10px;font-size:9px;color:#787F88;transition:transform .12s}
.dxsb-row.selected .dxsb-caret{color:#fff}
.dxsb-empty{padding:12px 10px;font-size:12.5px;color:#787F88;line-height:1.55}
.dxsb-hint{border-top:1px solid #E2E1DB;padding:10px 14px;font:11px/1.6 ui-sans-serif,system-ui;color:#787F88}
.dxsb-hint kbd{font:10px ui-monospace,monospace;background:#E7E6E1;border:1px solid #D7D6D0;border-bottom-width:2px;border-radius:3px;padding:0 4px}

.dxsb-main{display:flex;flex-direction:column;min-width:0;min-height:0}
.dxsb-toolbar{display:flex;align-items:center;gap:10px;padding:9px 18px;border-bottom:1px solid #E2E1DB;background:#fff;flex-shrink:0}
.dxsb-crumb{font-weight:600}
.dxsb-crumb .sep{color:#B8B7AF;margin:0 5px;font-weight:400}
.dxsb-spacer{flex:1}
.dxsb-id{font:11px ui-monospace,SFMono-Regular,monospace;color:#787F88}
.dxsb-tagrow{display:flex;gap:5px}
.dxsb-tag{font:10px ui-monospace,monospace;color:#4A5058;background:#F2F2EF;border:1px solid #E2E1DB;border-radius:3px;padding:0 5px}

.dxsb-canvas{flex:1;display:grid;place-items:center;padding:40px;background:#fff;overflow:auto;min-height:0}
.dxsb-blank{color:#787F88;font-size:13px;text-align:center;max-width:44ch;line-height:1.6}
.dxsb-blank code{font:12px ui-monospace,monospace;background:#F2F2EF;border:1px solid #E2E1DB;border-radius:4px;padding:1px 5px}
.dxsb-status{border-top:1px solid #E2E1DB;background:#F2F2EF;padding:6px 18px;font:11px ui-monospace,SFMono-Regular,monospace;color:#787F88;display:flex;gap:14px;flex-shrink:0}
.dxsb-status .warn{color:#9B1B1B}
"#;
