import { EditorState } from "@codemirror/state";
import { EditorView, drawSelection, keymap, placeholder } from "@codemirror/view";
import { defaultKeymap, history, historyKeymap } from "@codemirror/commands";
import { hybridPlugin } from "./hybrid";
import { axesKeymap } from "./axes";
import { chainKeymap, chainTrigger } from "./chainMenu";

export interface EditorOptions {
  doc: string;
  onChange(text: string): void;
  onSave(): void;
}

export function createEditor(parent: HTMLElement, opts: EditorOptions): EditorView {
  return new EditorView({
    parent,
    state: EditorState.create({
      doc: opts.doc,
      extensions: [
        history(),
        drawSelection(),
        EditorView.lineWrapping,
        placeholder("开始写作……"),
        chainKeymap,
        axesKeymap,
        keymap.of([
          {
            key: "Mod-s",
            run: () => {
              opts.onSave();
              return true;
            },
          },
          ...defaultKeymap,
          ...historyKeymap,
        ]),
        chainTrigger,
        hybridPlugin,
        EditorView.updateListener.of((u) => {
          if (u.docChanged) opts.onChange(u.state.doc.toString());
        }),
      ],
    }),
  });
}
