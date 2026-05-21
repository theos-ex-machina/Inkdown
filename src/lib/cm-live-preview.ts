import { syntaxTree } from "@codemirror/language";
import { RangeSetBuilder } from "@codemirror/state";
import {
  Decoration,
  EditorView,
  ViewPlugin,
  WidgetType,
  type DecorationSet,
  type ViewUpdate,
} from "@codemirror/view";
import katex from "katex";

/**
 * Obsidian-style live preview decorations for CodeMirror 6.
 *
 * The strategy is straightforward:
 * - Walk the visible syntax tree on every doc/viewport/selection change.
 * - For each "block" node (headings, blockquotes, code blocks, lists) attach a
 *   line-level class so CSS can give the line its rendered look.
 * - For each "marker" node (`#`, `**`, `_`, `` ` ``, `[`, `]`, `(`, `)`, `>`)
 *   that lives on a line where the cursor is NOT positioned, hide it with a
 *   mark decoration whose CSS sets `display: none`. As the cursor moves into
 *   the line the markers reappear, which is what makes "live preview" feel
 *   editable instead of WYSIWYG.
 * - Inline `$...$` math gets replaced by a KaTeX widget when not on the
 *   cursor line.
 *
 * If something doesn't parse (e.g. typing a half-finished `**bold`) we just
 * leave it alone — the syntax tree's untyped nodes won't match any of our
 * cases.
 */

const HIDE_MARK_CLASS = "cm-mark-hidden";

const BLOCK_LINE_CLASSES: Record<string, string> = {
  ATXHeading1: "cm-h1",
  ATXHeading2: "cm-h2",
  ATXHeading3: "cm-h3",
  ATXHeading4: "cm-h4",
  ATXHeading5: "cm-h5",
  ATXHeading6: "cm-h6",
  SetextHeading1: "cm-h1",
  SetextHeading2: "cm-h2",
  Blockquote: "cm-blockquote",
  HorizontalRule: "cm-hr",
};

const HIDDEN_MARK_NODES = new Set([
  "HeaderMark",
  "EmphasisMark",
  "CodeMark",
  "LinkMark",
  "ImageMark",
  "QuoteMark",
  "StrikethroughMark",
]);

const INLINE_HIDE_CLASS = new Set([
  "URL",
  "LinkTitle",
]);

class MathWidget extends WidgetType {
  constructor(
    private readonly src: string,
    private readonly display: boolean,
  ) {
    super();
  }
  eq(other: MathWidget): boolean {
    return other.src === this.src && other.display === this.display;
  }
  toDOM(): HTMLElement {
    const span = document.createElement(this.display ? "div" : "span");
    span.className = this.display ? "cm-math-block" : "cm-math-inline";
    try {
      span.innerHTML = katex.renderToString(this.src, {
        throwOnError: false,
        displayMode: this.display,
      });
    } catch {
      span.textContent = this.src;
    }
    return span;
  }
  ignoreEvent(): boolean {
    return false;
  }
}

function buildDecorations(view: EditorView): DecorationSet {
  const builder = new RangeSetBuilder<Decoration>();
  const doc = view.state.doc;
  const sel = view.state.selection.main;
  const cursorLine = doc.lineAt(sel.head).number;
  const selectionFrom = Math.min(sel.from, sel.head);
  const selectionTo = Math.max(sel.to, sel.head);

  type LineDeco = { class: string };
  const lineDecos = new Map<number, LineDeco>();

  type MarkDeco = { from: number; to: number; deco: Decoration };
  const markDecos: MarkDeco[] = [];

  for (const { from, to } of view.visibleRanges) {
    syntaxTree(view.state).iterate({
      from,
      to,
      enter: (node) => {
        const lineClass = BLOCK_LINE_CLASSES[node.name];
        if (lineClass) {
          const line = doc.lineAt(node.from);
          lineDecos.set(line.number, { class: lineClass });
        }
        // Fenced and indented code blocks: paint every line in the block.
        // We also tag the first/last lines so CSS can round just those
        // corners, which gives the box that GitHub / Obsidian feel.
        if (node.name === "FencedCode" || node.name === "CodeBlock") {
          const startLine = doc.lineAt(node.from).number;
          const endPos = Math.max(node.from, node.to - 1);
          const endLine = doc.lineAt(endPos).number;
          for (let i = startLine; i <= endLine; i++) {
            let cls = "cm-code-line";
            if (i === startLine) cls += " cm-code-line-first";
            if (i === endLine) cls += " cm-code-line-last";
            lineDecos.set(i, { class: cls });
          }
        }
        // Inline code: give the run a pill background. Backticks are
        // already hidden as CodeMark when off the cursor line.
        if (node.name === "InlineCode") {
          markDecos.push({
            from: node.from,
            to: node.to,
            deco: Decoration.mark({ class: "cm-inline-code" }),
          });
        }
        if (HIDDEN_MARK_NODES.has(node.name) || INLINE_HIDE_CLASS.has(node.name)) {
          const lineNo = doc.lineAt(node.from).number;
          const onCursorLine = lineNo === cursorLine;
          const inSelection = node.from < selectionTo && node.to > selectionFrom;
          if (!onCursorLine && !inSelection) {
            markDecos.push({
              from: node.from,
              to: node.to,
              deco: Decoration.mark({ class: HIDE_MARK_CLASS }),
            });
          }
        }
      },
    });
  }

  // Inline math: scan text manually since lang-markdown doesn't parse $..$.
  for (const { from, to } of view.visibleRanges) {
    const text = doc.sliceString(from, to);
    const inlineRe = /\$([^\s\$\\][^\$\n]*?[^\s\$\\]|[^\s\$\\])\$/g;
    let m: RegExpExecArray | null;
    while ((m = inlineRe.exec(text)) !== null) {
      const absFrom = from + m.index;
      const absTo = absFrom + m[0].length;
      // Skip if escaped
      if (absFrom > 0 && doc.sliceString(absFrom - 1, absFrom) === "\\") continue;
      const lineNo = doc.lineAt(absFrom).number;
      if (lineNo === cursorLine) continue;
      if (absFrom < selectionTo && absTo > selectionFrom) continue;
      markDecos.push({
        from: absFrom,
        to: absTo,
        deco: Decoration.replace({ widget: new MathWidget(m[1], false) }),
      });
    }
    const blockRe = /\$\$([\s\S]+?)\$\$/g;
    while ((m = blockRe.exec(text)) !== null) {
      const absFrom = from + m.index;
      const absTo = absFrom + m[0].length;
      const fromLine = doc.lineAt(absFrom).number;
      const toLine = doc.lineAt(absTo).number;
      if (cursorLine >= fromLine && cursorLine <= toLine) continue;
      if (absFrom < selectionTo && absTo > selectionFrom) continue;
      markDecos.push({
        from: absFrom,
        to: absTo,
        deco: Decoration.replace({ widget: new MathWidget(m[1], true), block: false }),
      });
    }
  }

  // Sort all decorations by `from` so the builder accepts them in order.
  const allDecos: { from: number; to: number; deco: Decoration; isLine: boolean }[] = [];
  for (const [lineNo, info] of lineDecos) {
    const line = doc.line(lineNo);
    allDecos.push({
      from: line.from,
      to: line.from,
      deco: Decoration.line({ class: info.class }),
      isLine: true,
    });
  }
  for (const m of markDecos) {
    allDecos.push({ from: m.from, to: m.to, deco: m.deco, isLine: false });
  }
  allDecos.sort((a, b) => {
    if (a.from !== b.from) return a.from - b.from;
    if (a.isLine && !b.isLine) return -1;
    if (!a.isLine && b.isLine) return 1;
    return a.to - b.to;
  });

  for (const d of allDecos) {
    builder.add(d.from, d.to, d.deco);
  }
  return builder.finish();
}

export const livePreview = ViewPlugin.fromClass(
  class {
    decorations: DecorationSet;
    constructor(view: EditorView) {
      this.decorations = buildDecorations(view);
    }
    update(u: ViewUpdate) {
      if (
        u.docChanged ||
        u.viewportChanged ||
        u.selectionSet ||
        u.geometryChanged
      ) {
        this.decorations = buildDecorations(u.view);
      }
    }
  },
  {
    decorations: (v) => v.decorations,
  },
);

export const livePreviewBaseTheme = EditorView.baseTheme({
  [`.${HIDE_MARK_CLASS}`]: {
    display: "none",
  },
  ".cm-h1": { fontSize: "1.9em", fontWeight: "700", lineHeight: "1.25", color: "var(--lavender)" },
  ".cm-h2": { fontSize: "1.55em", fontWeight: "700", lineHeight: "1.3", color: "var(--blue)" },
  ".cm-h3": { fontSize: "1.3em", fontWeight: "600", lineHeight: "1.35", color: "var(--sapphire)" },
  ".cm-h4": { fontSize: "1.15em", fontWeight: "600", color: "var(--teal)" },
  ".cm-h5": { fontSize: "1.05em", fontWeight: "600", color: "var(--green)" },
  ".cm-h6": { fontSize: "1em", fontWeight: "600", color: "var(--subtext1)" },
  ".cm-blockquote": {
    borderLeft: "3px solid var(--mauve)",
    paddingLeft: "0.8em",
    color: "var(--subtext1)",
    background: "rgba(203, 166, 247, 0.06)",
  },
  ".cm-hr": {
    borderTop: "1px solid var(--surface1)",
    color: "transparent",
    height: "0.5em",
  },
  ".cm-math-inline": {
    display: "inline-block",
    padding: "0 0.15em",
  },
  ".cm-math-block": {
    display: "inline-block",
    width: "100%",
    textAlign: "center",
    margin: "0.4em 0",
  },
  // Inline `code`: pill with subtle outline.
  ".cm-inline-code": {
    background: "var(--mantle)",
    border: "1px solid var(--surface0)",
    borderRadius: "4px",
    padding: "0.05em 0.35em",
    margin: "0 0.05em",
    fontFamily: "var(--mono)",
    fontSize: "0.9em",
    color: "var(--peach)",
  },
  // Fenced / indented code blocks: dark cutout that spans the editor width.
  ".cm-code-line": {
    background: "var(--mantle)",
    fontFamily: "var(--mono)",
    fontSize: "0.92em",
    color: "var(--text)",
    paddingLeft: "1em !important",
    paddingRight: "1em !important",
    borderLeft: "1px solid var(--surface0)",
    borderRight: "1px solid var(--surface0)",
  },
  ".cm-code-line-first": {
    borderTop: "1px solid var(--surface0)",
    borderTopLeftRadius: "var(--radius)",
    borderTopRightRadius: "var(--radius)",
    marginTop: "0.3em",
    paddingTop: "0.3em !important",
  },
  ".cm-code-line-last": {
    borderBottom: "1px solid var(--surface0)",
    borderBottomLeftRadius: "var(--radius)",
    borderBottomRightRadius: "var(--radius)",
    marginBottom: "0.3em",
    paddingBottom: "0.3em !important",
  },
});
