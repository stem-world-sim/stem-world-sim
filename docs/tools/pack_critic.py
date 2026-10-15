#!/usr/bin/env python3
"""Pack crates/critic/dist into one HTML file. No server needed."""
import base64, pathlib, sys

dist = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else "crates/critic/dist")
out = pathlib.Path(sys.argv[2] if len(sys.argv) > 2 else "critic.html")
js = next(p for p in dist.glob("*.js") if not p.name.endswith(".d.ts"))
wasm = next(dist.glob("*_bg.wasm"))
b64 = base64.b64encode(wasm.read_bytes()).decode()
glue = js.read_text()
# wasm-bindgen exports __wbg_init, often as default. Alias it.
glue = glue.replace("export default", "const init =").replace("export {", "const __keep = {")
page = f"""<!doctype html>
<meta charset="utf-8">
<title>stem-world-sim critic</title>
<style>html,body{{margin:0;height:100%;overflow:hidden}}#critic_canvas{{width:100%;height:100%}}</style>
<canvas id="critic_canvas"></canvas>
<script type="module">
{glue}
const start = (typeof init === "function") ? init : __wbg_init;
const bytes = Uint8Array.from(atob("{b64}"), c => c.charCodeAt(0));
await start({{module_or_path: bytes}});
</script>
"""
out.write_text(page)
print(out, out.stat().st_size)
