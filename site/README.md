# Kestrl website and docs

This folder is a plain static site. It has no build step, framework, external font, CDN or tracker.

Preview it from the repository root:

```sh
python3 -m http.server -d site 8765
```

Open <http://localhost:8765/>. The pages use one shared `style.css` and a small `script.js` that only powers the copy buttons. Icons live in `assets/` and are extracted from `app/Kestrl/Resources/AppIcon.icns`. All documentation can be read without JavaScript.

## Deploy

`scripts/deploy-site.sh` copies `site/` to the VPS and adds a content hash to the stylesheet and script links (`style.css?v=…`), so browsers and Cloudflare never pair new pages with an old cached stylesheet. The nginx config is `packaging/nginx/kestrl.adityakinjawadekar.com.conf`.
