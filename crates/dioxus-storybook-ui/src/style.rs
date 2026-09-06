//! The manager's stylesheet.
//!
//! Inlined rather than shipped as an asset so that a storybook is a single wasm
//! bundle with no extra fetch.
//!
//! There are two stylesheets because from M3 there are two documents. The
//! preview's iframe inherits nothing from the shell, which is the point — a
//! component under test must not be styled by the workbench around it — so the
//! handful of rules the canvas needs are restated there rather than shared.
//! They are not quite the same rules either: in the shell the canvas is one row
//! of a flex column, and in the frame it *is* the document.

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
/* Three columns: crumb | id | tools. `1fr auto 1fr` puts the id at the exact
   centre of the bar whatever sits either side of it, and pins the tools to the
   right edge — which is what keeps the Canvas/Docs toggle still while the
   viewport picker beside it appears and disappears. */
.dxsb-toolbar{display:grid;grid-template-columns:1fr auto 1fr;align-items:center;gap:10px;padding:9px 18px;border-bottom:1px solid #E2E1DB;background:#fff;flex-shrink:0}
.dxsb-crumb{font-weight:600;justify-self:start;min-width:0;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}
.dxsb-crumb .sep{color:#B8B7AF;margin:0 5px;font-weight:400}
.dxsb-tools{display:flex;align-items:center;gap:10px;justify-self:end;min-width:0}
.dxsb-id{justify-self:center;font:11px ui-monospace,SFMono-Regular,monospace;color:#787F88;white-space:nowrap}
/* --- globals toolbar --------------------------------------------------- */
.dxsb-globals{display:flex;align-items:center;gap:12px;padding-right:12px;border-right:1px solid #E2E1DB}
.dxsb-global{display:flex;align-items:center;gap:5px;font-size:12px;color:#4A5058;cursor:pointer}
.dxsb-global-title{font:11px ui-monospace,SFMono-Regular,monospace;color:#787F88;text-transform:uppercase;letter-spacing:.04em}
.dxsb-globals-input{padding:2px 5px;border:1px solid #CFCEC7;border-radius:5px;font:inherit;font-size:12px;background:#fff;color:#17191C}
.dxsb-globals-input:focus{outline:2px solid #C4451B;outline-offset:-1px;border-color:#C4451B}
.dxsb-globals-input.narrow{width:5.5em}
.dxsb-globals-range{width:90px;accent-color:#C4451B}
.dxsb-globals-reset{background:none;border:0;color:#787F88;font-size:13px;line-height:1;padding:3px 5px;border-radius:4px;cursor:pointer}
.dxsb-globals-reset:hover:not(:disabled){background:#E7E6E1;color:#C4451B}
.dxsb-globals-reset:disabled{opacity:.3;cursor:default}

/* --- canvas / docs toggle ---------------------------------------------- */
.dxsb-docstoggle{display:flex;align-items:center;gap:2px;padding:2px;background:#F2F2EF;border:1px solid #E2E1DB;border-radius:6px;flex-shrink:0}
.dxsb-docstab{background:none;border:0;border-radius:4px;padding:3px 10px;font:inherit;font-size:12px;color:#787F88;cursor:pointer}
.dxsb-docstab:hover{color:#17191C}
.dxsb-docstab.active{background:#fff;color:#17191C;font-weight:600;box-shadow:0 1px 2px rgba(23,25,28,.08)}

/* --- viewport picker --------------------------------------------------- */
.dxsb-viewport{display:flex;align-items:center;gap:8px;padding-right:12px;border-right:1px solid #E2E1DB}
.dxsb-viewport-rotate{background:none;border:0;color:#787F88;font-size:13px;line-height:1;padding:3px 5px;border-radius:4px;cursor:pointer}
.dxsb-viewport-rotate:hover:not(:disabled){background:#E7E6E1;color:#C4451B}
.dxsb-viewport-rotate:disabled{opacity:.3;cursor:default}
.dxsb-viewport-size{font:11px ui-monospace,SFMono-Regular,monospace;color:#787F88}

.dxsb-tagrow{display:flex;gap:5px}
.dxsb-tag{font:10px ui-monospace,monospace;color:#4A5058;background:#F2F2EF;border:1px solid #E2E1DB;border-radius:3px;padding:0 5px}

.dxsb-canvas{flex:1;display:grid;place-items:center;padding:40px;background:#fff;overflow:auto;min-height:0}
/* The pane the frame sits in. Responsive is the plain case: the frame is the
   pane. A chosen viewport makes the pane a scrollable backdrop with a
   fixed-size frame centred at the top of it — top, not middle, because a
   1112px-tall tablet in a 600px pane must start at its own beginning. */
.dxsb-stage{flex:1;min-height:0;display:flex;background:#fff}
.dxsb-stage.sized{overflow:auto;justify-content:center;align-items:flex-start;padding:18px;background:#F2F2EF}
.dxsb-frame{flex:1;min-height:0;width:100%;border:0;background:#fff;display:block}
.dxsb-stage.sized .dxsb-frame{flex:0 0 auto;width:auto;border:1px solid #E2E1DB;border-radius:4px;box-shadow:0 1px 5px rgba(23,25,28,.12)}
.dxsb-died{background:#FDF2F2;border-bottom:1px solid #E8C4C4;color:#9B1B1B;padding:10px 18px;flex-shrink:0}
.dxsb-died-head{display:flex;align-items:center;gap:12px;font-size:13px}
.dxsb-died-body{margin:6px 0 0;font:11.5px/1.5 ui-monospace,SFMono-Regular,monospace;white-space:pre-wrap;color:#7A1616;max-height:9em;overflow:auto}
.dxsb-blank{color:#787F88;font-size:13px;text-align:center;max-width:44ch;line-height:1.6}
.dxsb-blank code{font:12px ui-monospace,monospace;background:#F2F2EF;border:1px solid #E2E1DB;border-radius:4px;padding:1px 5px}
.dxsb-status{border-top:1px solid #E2E1DB;background:#F2F2EF;padding:6px 18px;font:11px ui-monospace,SFMono-Regular,monospace;color:#787F88;display:flex;gap:14px;flex-shrink:0}
.dxsb-status .warn{color:#9B1B1B}

/* --- addon panel ------------------------------------------------------- */
.dxsb-panel{border-top:1px solid #E2E1DB;background:#fff;display:flex;flex-direction:column;flex-shrink:0}
.dxsb-panel.open{height:38%;min-height:150px}
.dxsb-tabs{display:flex;align-items:stretch;gap:2px;padding:0 10px;border-bottom:1px solid #E2E1DB;background:#F2F2EF;flex-shrink:0}
.dxsb-tab{background:none;border:0;border-bottom:2px solid transparent;padding:8px 10px;font:inherit;font-size:12.5px;color:#787F88;cursor:pointer;display:flex;align-items:center;gap:6px}
.dxsb-tab:hover{color:#17191C}
.dxsb-tab.active{color:#17191C;font-weight:600;border-bottom-color:#C4451B}
.dxsb-count{background:#E2E1DB;color:#4A5058;border-radius:8px;padding:0 6px;font:10px ui-monospace,monospace}
.dxsb-tab.active .dxsb-count{background:#C4451B;color:#fff}
.dxsb-panelbtn{align-self:center;background:#fff;border:1px solid #CFCEC7;border-radius:5px;padding:3px 9px;font:inherit;font-size:11.5px;color:#4A5058;cursor:pointer;margin-left:8px}
.dxsb-panelbtn:hover:not(:disabled){border-color:#C4451B;color:#C4451B}
.dxsb-panelbtn:disabled{opacity:.45;cursor:default}
.dxsb-panelbody{flex:1;overflow:auto;padding:4px 0}
.dxsb-panelempty{color:#787F88;font-size:12.5px;padding:14px 18px;line-height:1.6;max-width:70ch}
.dxsb-panelempty code{font:11.5px ui-monospace,monospace;background:#F2F2EF;border:1px solid #E2E1DB;border-radius:4px;padding:1px 4px}
.dxsb-muted{color:#9A9A93;font-size:12px}

.dxsb-controls{width:100%;border-collapse:collapse;font-size:13px}
.dxsb-controls th{text-align:left;font:600 10px ui-monospace,monospace;letter-spacing:.1em;text-transform:uppercase;color:#9A9A93;padding:6px 12px;border-bottom:1px solid #E2E1DB}
.dxsb-controls th.dxsb-th-type{width:22%}
.dxsb-controls td{padding:7px 12px;border-bottom:1px solid #F0EFEA;vertical-align:top}
.dxsb-ctrl.overridden .dxsb-propname{color:#C4451B}
.dxsb-ctrlname{width:26%}
.dxsb-propname{font:500 13px ui-monospace,SFMono-Regular,monospace}
.dxsb-optional{color:#B8B7AF;font:12px ui-monospace,monospace}
.dxsb-propdocs{display:block;color:#787F88;font-size:11.5px;line-height:1.45;margin-top:2px}
/* `code` inside a quoted doc comment. See `inline.rs`. */
.dxsb-doccode{font:.92em ui-monospace,SFMono-Regular,monospace;background:#F2F2EF;border-radius:3px;padding:0 3px}
.dxsb-ctrlwidget{width:36%}
.dxsb-ctrltype code{font:11px ui-monospace,monospace;color:#787F88;word-break:break-all}
.dxsb-ctrlreset{width:44px;text-align:right}
.dxsb-unset{background:none;border:0;color:#C4451B;font-size:14px;cursor:pointer;padding:0 2px;line-height:1}
.dxsb-input{width:100%;max-width:280px;padding:4px 7px;border:1px solid #CFCEC7;border-radius:5px;font:inherit;font-size:12.5px;background:#fff}
.dxsb-input:focus{outline:2px solid #C4451B;outline-offset:-1px;border-color:#C4451B}
.dxsb-num{max-width:130px}
.dxsb-switch{display:inline-flex;align-items:center;gap:7px;font:12px ui-monospace,monospace;color:#4A5058}
.dxsb-rangewrap{display:flex;align-items:center;gap:10px;max-width:280px}
.dxsb-range{flex:1;accent-color:#C4451B}
.dxsb-readout{font:11.5px ui-monospace,monospace;color:#4A5058;min-width:44px;text-align:right}
.dxsb-radios{display:flex;flex-wrap:wrap;gap:4px 14px}
.dxsb-radio{display:inline-flex;align-items:center;gap:5px;font-size:12.5px;color:#4A5058}
.dxsb-colorwrap{display:flex;align-items:center;gap:8px}
.dxsb-colorwrap input[type=color]{width:32px;height:26px;padding:1px;border:1px solid #CFCEC7;border-radius:5px;background:#fff}
.dxsb-hex{max-width:110px;font:12px ui-monospace,monospace}

.dxsb-actions{list-style:none;margin:0;padding:0;font:12px ui-monospace,SFMono-Regular,monospace}
.dxsb-actions li{display:flex;gap:10px;padding:5px 14px;border-bottom:1px solid #F0EFEA}
.dxsb-actionname{color:#C4451B;font-weight:600;flex-shrink:0;min-width:9ch}
.dxsb-actionpayload{color:#4A5058;white-space:pre-wrap;word-break:break-word}
"#;

/// The preview document's stylesheet.
///
/// Everything the story canvas needs and nothing else. Whatever the story
/// itself brings — a component's own `style` attribute, a `document::Link` in a
/// decorator — lands on top of this.
pub const PREVIEW_CSS: &str = r#"
*{box-sizing:border-box}
html,body{margin:0;height:100%}
body{font:14px/1.5 ui-sans-serif,system-ui,-apple-system,sans-serif;color:#17191C;background:#fff}
/* 100vh, not 100%: a percentage height only resolves against an ancestor chain
   that has one, and the canvas sits inside whatever element the host app
   mounted into. The frame's viewport is the one height always known. */
.dxsb-canvas{min-height:100vh;display:grid;place-items:center;padding:40px;overflow:auto}
/* `parameters.layout`, the one decision a story makes about its surface. */
.dxsb-canvas.layout-padded{place-items:start center}
.dxsb-canvas.layout-fullscreen{padding:0;place-items:stretch}
.dxsb-blank{color:#787F88;font-size:13px;text-align:center;max-width:44ch;line-height:1.6}
.dxsb-blank code{font:12px ui-monospace,monospace;background:#F2F2EF;border:1px solid #E2E1DB;border-radius:4px;padding:1px 5px}

/* --- the autodocs page -------------------------------------------------
   A document, not a canvas: it scrolls, it has a reading measure, and it is
   the one thing in the preview that is chrome rather than component. It lives
   here rather than in the manager's stylesheet because it renders in the
   frame — which it must, since every example on it is a real story with its
   own decorators. */
.dxsb-docs{max-width:920px;margin:0 auto;padding:44px 32px 96px;color:#17191C}
.dxsb-docs-head{border-bottom:1px solid #E2E1DB;padding-bottom:22px;margin-bottom:8px}
.dxsb-docs-path{margin:0;font:11px ui-monospace,SFMono-Regular,monospace;letter-spacing:.08em;text-transform:uppercase;color:#9A9A93}
.dxsb-docs-title{margin:6px 0 0;font-size:30px;line-height:1.15;letter-spacing:-.01em}
.dxsb-docs-lede{margin:12px 0 0;font-size:15px;line-height:1.65;color:#4A5058;max-width:68ch}
.dxsb-docs-note{margin:24px 0 0;font-size:13px;color:#787F88;line-height:1.6}
.dxsb-docs-note code{font:12px ui-monospace,monospace;background:#F2F2EF;border:1px solid #E2E1DB;border-radius:4px;padding:1px 5px}
.dxsb-docs-h2{font-size:12px;font-weight:600;letter-spacing:.11em;text-transform:uppercase;color:#9A9A93;margin:40px 0 12px}
.dxsb-docs-h3{font-size:17px;margin:0 0 4px;letter-spacing:-.005em}
.dxsb-docs-blurb{margin:0 0 14px;font-size:13.5px;line-height:1.6;color:#4A5058;max-width:68ch}

/* The props table. Deliberately not `.dxsb-controls`: that one is a panel of
   editable widgets in a 38%-tall drawer, and this is a reference table on a
   page. Same data, different job. */
.dxsb-proptable{width:100%;border-collapse:collapse;font-size:13px;table-layout:fixed}
.dxsb-proptable th{text-align:left;font:600 10px ui-monospace,monospace;letter-spacing:.1em;text-transform:uppercase;color:#9A9A93;padding:7px 12px;border-bottom:1px solid #E2E1DB}
.dxsb-proptable th:nth-child(1){width:20%}
.dxsb-proptable th:nth-child(2){width:26%}
.dxsb-proptable th:nth-child(3){width:18%}
.dxsb-proptable td{padding:9px 12px;border-bottom:1px solid #F0EFEA;vertical-align:top;line-height:1.5}
.dxsb-propname{font:500 13px ui-monospace,SFMono-Regular,monospace}
.dxsb-optional{color:#B8B7AF;font:12px ui-monospace,monospace}
.dxsb-proptype,.dxsb-propdefault{font:11.5px ui-monospace,SFMono-Regular,monospace;color:#787F88;word-break:break-word}
.dxsb-propdefault{color:#C4451B}
.dxsb-propcell-docs{color:#4A5058;font-size:12.5px}
.dxsb-doccode{font:.92em ui-monospace,SFMono-Regular,monospace;background:#F2F2EF;border-radius:3px;padding:0 3px}
.dxsb-muted{color:#B8B7AF;font-size:12px}

/* Each story: the live example above, the code that made it below. */
.dxsb-docs-story{margin:0 0 40px}
.dxsb-docs-example{border:1px solid #E2E1DB;border-radius:7px;background:#fff;padding:28px;display:flex;justify-content:center;align-items:center;min-height:96px;overflow:auto}
.dxsb-docs-source{margin-top:8px}
.dxsb-docs-source summary{font-size:11.5px;color:#787F88;cursor:pointer;padding:4px 2px;list-style:none;user-select:none}
.dxsb-docs-source summary::-webkit-details-marker{display:none}
.dxsb-docs-source summary::before{content:"\25B8 ";color:#B8B7AF}
.dxsb-docs-source[open] summary::before{content:"\25BE "}
.dxsb-docs-source summary:hover{color:#C4451B}
.dxsb-docs-source pre{margin:4px 0 0;background:#17191C;color:#EDEDEA;border-radius:7px;padding:16px 18px;overflow-x:auto}
.dxsb-docs-source code{font:12px/1.6 ui-monospace,SFMono-Regular,monospace;white-space:pre}
.dxsb-docs-id{margin:8px 0 0;font:10.5px ui-monospace,SFMono-Regular,monospace;color:#C9C8C1}
"#;
