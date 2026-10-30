---
name: architecture-diagram
description: Create or update architecture and flow diagrams for agent-desktop's docs site in apps/docs. Use when a docs page needs a diagram of components, data flow, or a command pipeline, or when an existing diagram in apps/docs/public/desktop/diagrams changes.
metadata:
  internal: true
---

# Architecture diagram

Diagrams are standalone HTML files that the docs embed with the `Diagram` component. Follow the technical-writer skill for any prose.

## Folder convention

- One folder per diagram: `apps/docs/public/desktop/diagrams/<name>/index.html`, served at `/desktop/diagrams/<name>/index.html`. `<name>` is a short kebab-case slug. Keep it under `/desktop`: agent-browser.dev routes only `/desktop` paths to this app.
- `apps/docs/proxy.ts` returns 404 for `/desktop` paths that are not docs pages, so its matcher must exclude `desktop/diagrams/`, as it does for `desktop/favicon.ico`.
- The file is standalone: inline CSS, one inline SVG, no external requests. Start from [references/template.html](references/template.html).
- The visual tokens are in [references/style.md](references/style.md). Reuse them so every diagram looks the same.
- The theme comes from `?theme=light` or `?theme=dark`. Without the parameter the diagram follows `prefers-color-scheme` and defaults to dark.
- Give the SVG a `<title>` and a `<desc>` that states the whole flow in words.

## Draw the real mechanism

1. Read the code before drawing. For observation and refs, read `crates/core/src/snapshot/`, `crates/core/src/refs/`, and the command in `crates/core/src/commands/`. For platforms, read the `PlatformAdapter` trait in `crates/core/src/adapter/` and the platform crate. Draw only components, names, and directions you can point to in source.
2. Use the identifiers from the code as labels (`PlatformAdapter`, `RefStore`) and put the mechanism in the gray second line (`AX API`, `UI Automation`).
3. Draw one arrow per real data path. Label arrows that cross a boundary with the command, flag, or call (`snapshot --root`).
4. Group components by where they run: an outer dashed container for a machine, a nested panel for a process. Put the calling agent outside.
5. Mark the entry point of the flow with the highlighted circle node. Use it once.
6. Lay it out for the 720px docs column at 1:1 scale (see style.md) and keep it to 12 nodes or fewer. Move detail to the page text.

## Embed it

```mdx
<Diagram name="snapshot-drill-down" title="Skeleton snapshot and drill-down" aspectRatio="720 / 448" />
```

`aspectRatio` is the SVG `viewBox` width and height. The component passes the site theme to the iframe.

If `apps/docs` has no `Diagram` component yet, add `components/geistdocs/diagram.tsx`: a client component that renders the file in a lazy iframe from `/desktop/diagrams/<name>/index.html` and appends `?theme=` from `next-themes`. Register it in `components/geistdocs/mdx-components.tsx`.

## Text version

Keep a text version of the flow next to the diagram on the page, such as a short list or code block with the same components and order. Readers of the `.md` route, agents, and screen readers depend on it.

## Check it

1. In `apps/docs`, run `pnpm dev` and, in a second terminal, `pnpm exec microfrontends proxy microfrontends.json --local-apps agent-desktop`.
2. Open `http://localhost:3024/desktop/diagrams/<name>/index.html` and the same URL with `?theme=light` at 1440x900. Look for clipped text, overlapping labels, arrows that miss their node, and unreadable gray text.
3. Open the docs page and check the diagram in both site themes.
4. Run `pnpm build` in `apps/docs`.
5. Stop the dev server and the proxy, and close the browser session.
