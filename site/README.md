# Kestrl website and docs

This folder is a plain static site. It has no build step, framework, external font, CDN or tracker.

Preview it from the repository root:

```sh
python3 -m http.server -d site 8765
```

Open <http://localhost:8765/>. The pages use one shared `style.css` and a small `script.js` for code copy buttons and the actor example on the landing page. All documentation can be read without JavaScript.
