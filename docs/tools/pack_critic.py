#!/usr/bin/env python3
"""Pack crates/critic/dist into one HTML file. No server needed."""
import base64, pathlib, re, sys

dist = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else "crates/critic/dist")
out = pathlib.Path(sys.argv[2] if len(sys.argv) > 2 else "critic.html")
html = (dist / "index.html").read_text()
js = next(dist.glob("*.js"))
wasm = next(dist.glob("*_bg.wasm"))
b64 = base64.b64encode(wasm.read_bytes()).decode()
glue = js.read_text()
# trunk loads the js as a module that fetches the wasm; hand it bytes instead
page = f"""<!doctype html>
<meta charset="utf-8">
<title>stem-world-sim critic</title>
<canvas id="critic_canvas"></canvas>
<script type="module">
{glue}
const bytes = Uint8Array.from(atob("{b64}"), c => c.charCodeAt(0));
await init(bytes);
</script>
"""
out.write_text(page)
print(out, out.stat().st_size)
