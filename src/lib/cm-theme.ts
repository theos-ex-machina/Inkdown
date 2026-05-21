import { EditorView } from "@codemirror/view";
import { HighlightStyle, syntaxHighlighting } from "@codemirror/language";
import { tags as t } from "@lezer/highlight";

const palette = {
  base: "#1e1e2e",
  mantle: "#181825",
  surface0: "#313244",
  surface1: "#45475a",
  text: "#cdd6f4",
  subtext0: "#a6adc8",
  subtext1: "#bac2de",
  overlay1: "#7f849c",
  lavender: "#b4befe",
  blue: "#89b4fa",
  sapphire: "#74c7ec",
  teal: "#94e2d5",
  green: "#a6e3a1",
  yellow: "#f9e2af",
  peach: "#fab387",
  maroon: "#eba0ac",
  red: "#f38ba8",
  mauve: "#cba6f7",
  pink: "#f5c2e7",
  rosewater: "#f5e0dc",
};

export const catppuccinTheme = EditorView.theme(
  {
    "&": {
      color: palette.text,
      backgroundColor: palette.base,
      fontFamily:
        '"JetBrains Mono", "Fira Code", "Cascadia Code", Consolas, monospace',
      fontSize: "14px",
      height: "100%",
    },
    ".cm-content": {
      caretColor: palette.lavender,
      padding: "20px 24px 80px",
    },
    ".cm-scroller": {
      fontFamily: "inherit",
      lineHeight: "1.55",
    },
    "&.cm-focused .cm-cursor": {
      borderLeftColor: palette.lavender,
    },
    "&.cm-focused .cm-selectionBackground, ::selection, .cm-selectionBackground":
      {
        background: palette.surface1,
      },
    ".cm-gutters": {
      backgroundColor: palette.mantle,
      color: palette.overlay1,
      border: "none",
    },
    ".cm-activeLine": {
      backgroundColor: "rgba(255,255,255,0.02)",
    },
    ".cm-activeLineGutter": {
      backgroundColor: palette.surface0,
    },
    ".cm-matchingBracket, .cm-nonmatchingBracket": {
      color: palette.peach,
      outline: `1px solid ${palette.surface1}`,
    },
    ".cm-tooltip": {
      background: palette.surface0,
      border: `1px solid ${palette.surface1}`,
      color: palette.text,
    },
    ".cm-panels": {
      background: palette.mantle,
      color: palette.text,
    },
    ".cm-panel.cm-search input": {
      background: palette.surface0,
      color: palette.text,
      border: `1px solid ${palette.surface1}`,
      borderRadius: "4px",
      padding: "2px 6px",
    },
    ".cm-link": {
      color: palette.sapphire,
    },
  },
  { dark: true },
);

export const catppuccinHighlight = HighlightStyle.define([
  { tag: t.heading, color: palette.lavender, fontWeight: "600" },
  { tag: t.heading1, color: palette.lavender, fontWeight: "700" },
  { tag: t.heading2, color: palette.blue, fontWeight: "600" },
  { tag: t.heading3, color: palette.sapphire, fontWeight: "600" },
  { tag: t.strong, color: palette.peach, fontWeight: "700" },
  { tag: t.emphasis, color: palette.maroon, fontStyle: "italic" },
  { tag: t.link, color: palette.sapphire, textDecoration: "underline" },
  { tag: t.url, color: palette.teal },
  { tag: t.monospace, color: palette.peach },
  { tag: t.keyword, color: palette.mauve },
  { tag: t.atom, color: palette.peach },
  { tag: t.string, color: palette.green },
  { tag: t.number, color: palette.peach },
  { tag: t.comment, color: palette.overlay1, fontStyle: "italic" },
  { tag: t.meta, color: palette.overlay1 },
  { tag: t.punctuation, color: palette.subtext0 },
  { tag: t.bracket, color: palette.subtext0 },
  { tag: t.variableName, color: palette.text },
  { tag: t.list, color: palette.yellow },
  { tag: t.quote, color: palette.subtext0 },
  { tag: t.contentSeparator, color: palette.overlay1 },
  { tag: t.processingInstruction, color: palette.pink },
]);

export const catppuccinExtension = [
  catppuccinTheme,
  syntaxHighlighting(catppuccinHighlight),
];
