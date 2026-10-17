export interface FileNode {
  name: string;
  path: string;
  kind: "dir" | "file";
  children?: FileNode[];
}

export interface OpenDoc {
  path: string;
  name: string;
  content: string;
  saved: string;
}

export interface OutlineItem {
  level: number;
  text: string;
  pos: number;
}

export interface FolderRoot {
  name: string;
  path: string;
  tree: FileNode;
}

export interface TemplateFile {
  group: "docs" | "menus" | "chains";
  name: string;
  path: string;
}
