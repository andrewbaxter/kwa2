importScripts("./serviceworker.js", "./serviceworker_bg_wasm.js");
const ready = wasm_bindgen({ module_or_path: serviceworker_bg_wasm });
addEventListener("install", (ev) =>
  ev.waitUntil(ready.then(() => wasm_bindgen.handle_install())),
);
addEventListener("activate", (ev) =>
  ev.waitUntil(ready.then(() => wasm_bindgen.handle_activate())),
);
addEventListener("fetch", (ev) =>
  ev.respondWith(ready.then(() => wasm_bindgen.handle_fetch(ev))),
);
addEventListener("push", (ev) =>
  ev.waitUntil(ready.then(() => wasm_bindgen.handle_push(ev))),
);
