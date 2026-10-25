# Kestrl website and docs

This folder is a plain static site. It has no build step, framework, external font, CDN or tracker.

Preview it from the repository root:

```sh
python3 -m http.server -d site 8765
```

Open <http://localhost:8765/>. The pages use one shared `style.css` and a small `script.js` that only powers the copy buttons. Icons live in `assets/` and are extracted from `app/Kestrl/Resources/AppIcon.icns`. All documentation can be read without JavaScript.
