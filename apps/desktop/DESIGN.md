# Glyphibe desktop design

The product is a code-comprehension instrument. The graph is the main working surface; every overlay earns its space through navigation, evidence or teaching.

Tokens: graphite `#171d26`, slate glass `#25313e`, paper `#edf3f7`, muted `#a6b9c9`, relation blue `#72b4df`, inferred amber `#e5bc70`. Light mode uses pale grey surfaces and dark graphite text. Segoe UI Variable provides native Windows reading; Cascadia Mono is reserved for source and exact symbols. System fallbacks keep the application offline.

Layout: full viewport canvas, native-feeling narrow title strip, top-center graph-mode island, left search/navigation island, bottom granularity controls, contextual right evidence/teaching panel. Text and forms align left. Overlay glass can be disabled; pinning disables pointer-out auto collapse. Focus mode hides all islands except a recovery button.

```
 title / repository / open / focus
             mode island
 search |          graph          | evidence
 tools  |      shared selection   | lesson
           granularity / density
```

Review against brief: a dashboard grid would reduce the graph area, so no metric cards or chat column are used. The memorable element is the actual code relationship map. Colour identifies entity families and evidence, not decoration. Native Mica is attempted on supported Windows builds and an opaque fallback remains usable. Reduced motion disables camera animation and force warm-up movement.

Accessibility: visible focus, labels for every control, searchable keyboard-operable entity list as a canvas alternative, readable contrast, explicit uncertainty and stale evidence, Escape/F focus controls, theme and opacity toggles. Imported graph content is untrusted text and never rendered as HTML.
